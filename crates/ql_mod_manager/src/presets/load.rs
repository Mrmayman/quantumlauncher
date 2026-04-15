use std::{
    io::{Cursor, Read},
    path::Path,
    sync::Arc,
};

use owo_colors::OwoColorize;
use ql_core::{Instance, IntoIoError, IntoJsonError, Loader, info, json::VersionDetails, pt};

use crate::{
    presets::{OVERRIDES_NAME, PresetJson, get_instance_type},
    store::{ModError, ModId},
};

#[must_use]
#[derive(Debug, Clone, Default)]
pub struct PresetOutput {
    pub instance_name: Arc<str>,
    pub is_server: bool,

    pub local_mods: Vec<String>,
    pub local_overrides: Vec<String>,
    pub to_install: Vec<ModId>,
    pub is_regular_modpack: bool,

    pub game_version: String,
    pub mod_type: Loader,
}

/// Installs or previews a `.qmp` preset file.
///
/// # Arguments
/// - `instance`: target instance
/// - `file`: `.qmp` file bytes
/// - `apply`: whether to install or just preview
///
/// Returns mod IDs for installation with [`crate::store::download_mods_bulk`].
///
/// # Errors
/// - Invalid zip file or JSON
/// - Permission or path access issues
/// - Instance configuration errors
pub async fn load(instance: Instance, file: &[u8], apply: bool) -> Result<PresetOutput, ModError> {
    info!("Importing mod preset");

    let dotmc_dir = instance.get_dot_minecraft_path();
    let mods_dir = dotmc_dir.join("mods");

    let mut zip = zip::ZipArchive::new(Cursor::new(file)).map_err(ModError::Zip)?;

    let version_json = VersionDetails::load(&instance).await.ok();
    let instance_type = get_instance_type(&instance).await.ok();

    let mut local_mods = Vec::new();
    let mut local_overrides = Vec::new();

    let index: PresetJson = {
        let Ok(mut index) = zip.by_name("index.json") else {
            // Else this ain't a QMP file!
            // Install as regular modpack
            return Ok(PresetOutput {
                is_server: instance.is_server(),
                instance_name: instance.name,
                local_mods: Vec::new(),
                local_overrides: Vec::new(),
                to_install: Vec::new(),
                is_regular_modpack: true,
                game_version: version_json
                    .map(|n| n.get_id().to_owned())
                    .unwrap_or_default(),
                mod_type: instance_type.unwrap_or(Loader::Vanilla),
            });
        };
        let buf = std::io::read_to_string(&mut index)
            .map_err(|n| ModError::ZipIoError(n, "index.json".to_owned()))?;
        serde_json::from_str(&buf).json(buf)?
    };

    // Only sideload mods if the version is the same
    let should_sideload = version_json.is_some_and(|n| n.get_id() == index.minecraft_version)
        && instance_type.is_some_and(|n| n == index.instance_type);

    for i in 0..zip.len() {
        let mut file = zip.by_index(i).map_err(ModError::Zip)?;

        process_file(
            apply,
            should_sideload,
            &dotmc_dir,
            &mods_dir,
            &mut file,
            &mut local_mods,
            &mut local_overrides,
        )
        .await?;
    }

    let to_install = index
        .entries_downloaded
        .into_iter()
        .filter_map(|(k, n)| n.manually_installed.then_some(k))
        .collect();

    Ok(PresetOutput {
        is_server: index.is_server.unwrap_or(instance.is_server()),
        instance_name: index.instance_name.unwrap_or_else(|| instance.name),
        local_mods,
        local_overrides,
        to_install,
        is_regular_modpack: false,
        game_version: index.minecraft_version,
        mod_type: index.instance_type,
    })
}

async fn process_file(
    apply: bool,
    should_sideload: bool,
    dotmc_dir: &Path,
    mods_dir: &Path,
    file: &mut zip::read::ZipFile<'_, Cursor<&[u8]>>,
    local_mods: &mut Vec<String>,
    local_overrides: &mut Vec<String>,
) -> Result<(), ModError> {
    // QMP files contain the following features
    //
    // 1) `index.json` - Metadata/manifest,
    //    already loaded so can be skipped
    // 2) `config/*` - Mod configuration files
    //    - Deprecated in favor of `overrides/config/`
    //    - Still supported for backwards compatibility
    // 3) `overrides/*` - Things to add to .minecraft folder
    // 4) `*.jar` in root - Locally added mods

    let name = file.name();

    if name == "index.json" { // 1
    } else if name.starts_with("config/") || name.starts_with("config\\") {
        // 2
        if !apply {
            return Ok(());
        }
        if !name.ends_with('/') && !name.ends_with('\\') {
            pt!("Config: {}", name.bright_black());
        }
        write_file(dotmc_dir, file, &name.to_owned()).await?;
    } else if name.starts_with(constcat::concat!(OVERRIDES_NAME, "/"))
        || name.starts_with(constcat::concat!(OVERRIDES_NAME, "\\"))
    {
        // 3
        if !apply {
            return Ok(());
        }
        let name = name.replace('\\', "/");
        let name = name.strip_prefix(OVERRIDES_NAME).unwrap_or(&name);
        let name = name.strip_prefix("/").unwrap_or(name);
        if !name.ends_with('/') {
            pt!("Override: {}", name.bright_black());
            local_overrides.push(name.to_owned());
        }
        if !should_sideload {
            return Ok(());
        }
        write_file(dotmc_dir, file, &name).await?;
    } else if name.contains('/') || name.contains('\\') {
        info!("Feature not implemented: {name}");
    } else {
        // 4
        if !should_sideload {
            return Ok(());
        }
        local_mods.push(name.to_owned());
        if !apply {
            return Ok(());
        }

        pt!("Local file: {name}");
        let path = mods_dir.join(&name);
        let mut buf = Vec::new();
        let name = name.to_owned();
        file.read_to_end(&mut buf)
            .map_err(|n| ModError::ZipIoError(n, name))?;
        tokio::fs::write(&path, &buf).await.path(&path)?;
    }
    Ok(())
}

async fn write_file(
    root_dir: &Path,
    file: &mut zip::read::ZipFile<'_, Cursor<&[u8]>>,
    name: &str,
) -> Result<(), ModError> {
    let path = root_dir.join(name.replace('\\', "/"));
    if file.is_dir() {
        tokio::fs::create_dir_all(&path).await.path(&path)?;
    } else {
        let parent = path.parent().unwrap();
        tokio::fs::create_dir_all(parent).await.path(parent)?;

        let mut buf = Vec::new();
        file.read_to_end(&mut buf)
            .map_err(|n| ModError::ZipIoError(n, name.to_owned()))?;
        tokio::fs::write(&path, &buf).await.path(&path)?;
    }
    Ok(())
}
