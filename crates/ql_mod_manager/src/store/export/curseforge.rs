use std::{collections::HashSet, path::PathBuf};

use ql_core::{Instance, json::VersionDetails};
use serde::Serialize;

use crate::store::{
    ModId, ModIndex,
    export::{create_override_mods_full_path, overrides_fn, package_format1_pack},
};

#[derive(Serialize)]
struct CurseForgeModpackManifest {
    minecraft: CurseForgeMinecraftConfig,
    manifest_type: String,
    manifest_version: u32,
    name: String,
    version: String,
    author: String,
    files: Vec<CurseForgeFileEntry>,
    overrides: String,
    image: String,
}

#[derive(Serialize)]
struct CurseForgeMinecraftConfig {
    version: String,
    mod_loaders: Vec<CurseForgeModLoader>,
}

#[derive(Serialize)]
struct CurseForgeModLoader {
    id: String,
    primary: bool,
}

#[derive(Serialize)]
struct CurseForgeFileEntry {
    project_id: u64,
    file_id: u64,
    required: bool,
}

pub async fn export_curseforge_modpack(
    author: String,
    modpack_name: String,
    modpack_version: String,
    modpack_file_name: String,
    mod_ids: HashSet<ModId>,
    modpack_path: String,
    overrides: Vec<String>, // MUST BE FULL PATH!!
    icon_url: String,
    instance: Instance,
) {
    let index = ModIndex::load(&instance).await.unwrap();

    let mut override_filenames: Vec<String> = Vec::new();

    for id in &mod_ids {
        let is_curseforge = matches!(id, ModId::Curseforge(_));

        let Some(config) = index.mods.get(id) else {
            continue;
        };
        let Some(primary_file) = config
            .files
            .iter()
            .find(|file| file.primary)
            .or_else(|| config.files.first())
        else {
            continue;
        };

        if is_curseforge {
            continue;
        } else {
            override_filenames.push(primary_file.filename.clone());
        }
    }

    let details = VersionDetails::load(&instance).await.unwrap();
    let minecraft_version = details.get_id().to_string();
    let config = ql_core::InstanceConfigJson::read(&instance).await;
    let loader_name = match config.as_ref().unwrap().mod_type.to_curseforge_num() {
        "1" => "forge",
        "4" => "fabric",
        "5" => "quilt",
        "6" => "neoforge",
        "3" => "lightloader",
        _ => panic!(),
    };
    let loader_version = config.unwrap().mod_type_info.unwrap().version;
    let loader = loader_name.to_string() + "-" + loader_version.unwrap().as_str();

    let file_ids: Vec<&str> = vec!["temp"]; // TODO: get FileIds here!!

    let mod_ids: Vec<String> = mod_ids
        .into_iter()
        .map(|map| serde_json::to_string(&map).unwrap())
        .collect();

    let mods_folder_path = instance.get_dot_minecraft_path().join("mods");
    let override_mods_full_path_string: Vec<String> =
        create_override_mods_full_path(override_filenames, &mods_folder_path);

    let zip_path = PathBuf::from(&modpack_path)
        .join(format!("{}.zip", modpack_file_name))
        .to_string_lossy()
        .to_string();

    let overrides: Vec<(String, String)> =
        overrides_fn(override_mods_full_path_string, overrides, instance);

    let json_data = write_curseforge_manifest_json(
        mod_ids,
        file_ids,
        author,
        modpack_version,
        modpack_name,
        loader,
        minecraft_version,
        icon_url,
    )
    .unwrap();

    package_format1_pack("manifest".to_string(), json_data, zip_path, overrides)
        .await
        .unwrap();
}

fn write_curseforge_manifest_json(
    mod_id: Vec<String>,
    file_id: Vec<&str>,
    author: String,
    modpack_version: String,
    name: String,
    loader_id: String,
    version: String,
    image: String,
) -> std::io::Result<String> {
    let primary = true;

    let files: Vec<CurseForgeFileEntry> = mod_id
        .into_iter()
        .zip(file_id.into_iter())
        .map(|(proj_str, file_str)| CurseForgeFileEntry {
            project_id: proj_str.parse::<u64>().unwrap(),
            file_id: file_str.parse::<u64>().unwrap(),
            required: true,
        })
        .collect();

    let manifest = CurseForgeModpackManifest {
        minecraft: CurseForgeMinecraftConfig {
            version,
            mod_loaders: vec![CurseForgeModLoader {
                id: loader_id,
                primary,
            }],
        },
        manifest_type: "minecraftModpack".to_string(),
        manifest_version: 1,
        name,
        version: modpack_version,
        author,
        files,
        overrides: "overrides".to_string(),
        image,
    };

    let manifest_json = serde_json::to_string_pretty(&manifest)?;

    Ok(manifest_json)
}
