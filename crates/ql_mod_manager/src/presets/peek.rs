use std::io::Cursor;

use ql_core::{IntoJsonError, info};

use crate::{
    presets::{OVERRIDES_NAME, PresetJson},
    store::{
        ModError,
        modpack::{self, PeekInfo},
    },
};

/// Previews a `.qmp` preset without requiring an instance.
///
/// Unlike [`load`], this:
/// - does not install anything
/// - does not need an [`Instance`]
/// - only extracts metadata / preview info
///
/// # Errors
/// - Invalid zip file or JSON
pub fn peek(file: &[u8]) -> Result<Option<PeekInfo>, ModError> {
    info!("Previewing mod preset");

    let mut zip = zip::ZipArchive::new(Cursor::new(file)).map_err(ModError::Zip)?;

    let mut local_mods = Vec::new();
    let mut local_overrides = Vec::new();

    let index: PresetJson = {
        let Ok(mut index) = zip.by_name("index.json") else {
            // Will not recurse; modpack::peek will only call this
            // if there is an index.json
            return modpack::peek(file);
        };

        let buf = std::io::read_to_string(&mut index)
            .map_err(|n| ModError::ZipIoError(n, "index.json".to_owned()))?;

        serde_json::from_str(&buf).json(buf)?
    };

    for i in 0..zip.len() {
        let file = zip.by_index(i).map_err(ModError::Zip)?;
        let name = file.name();

        if name == "index.json" {
            continue;
        }

        // overrides/*
        if name.starts_with(constcat::concat!(OVERRIDES_NAME, "/"))
            || name.starts_with(constcat::concat!(OVERRIDES_NAME, "\\"))
        {
            let name = name.replace('\\', "/");
            let name = name.strip_prefix(OVERRIDES_NAME).unwrap_or(&name);
            let name = name.strip_prefix('/').unwrap_or(name);

            if !name.ends_with('/') {
                local_overrides.push(name.to_owned());
            }
        }
        // root-level jars/files
        else if !name.contains('/') && !name.contains('\\') {
            local_mods.push(name.into());
        }
    }

    Ok(Some(PeekInfo {
        name: index
            .metadata
            .as_ref()
            .and_then(|n| n.nice_name.clone())
            .or(index.instance_name.clone())
            .unwrap_or_else(|| "QMP Modpack".into()),
        game_version: index.minecraft_version,
        loader: index.instance_type,

        local_mods,
        download_mods: Some(
            index
                .entries_downloaded
                .iter()
                .filter(|n| n.1.manually_installed)
                .map(|(id, c)| (id.clone(), c.name.clone()))
                .collect(),
        ),
        recommended_ram_mb: None,
        local_overrides,
    }))
}
