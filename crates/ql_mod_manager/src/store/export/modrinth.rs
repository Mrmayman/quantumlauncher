use std::{collections::HashSet, path::PathBuf};

use ql_core::{Instance, json::VersionDetails};
use serde::Serialize;
use serde_json::{Map, Value};
use std::io::Result as StdResult;

use crate::store::{
    ModId, ModIndex,
    export::{
        create_override_mods_full_path, hash_file,
        overrides_fn, package_format_1_modpack,
    },
};
use crate::store::export::Hashes;

#[derive(Serialize)]
pub struct ModrinthModpackManifest {
    #[serde(rename = "formatVersion")]
    format_version: u8,
    game: String,
    #[serde(rename = "versionId")]
    version_id: String,
    name: String,
    summary: String,
    files: Vec<FormatMQFileEntry>,
    dependencies: Value,
}

#[derive(Serialize)]
pub struct FormatMQFileEntry {
    path: String,
    hashes: Hashes,
    #[serde(rename = "downloads")]
    downloads: Vec<String>,
    #[serde(rename = "fileSize")]
    file_size: u64,
}

pub async fn export_modrinth_modpack(
    modpack_path: String,
    modpack_name: String,
    modpack_version: String,
    modpack_summary: String,
    modpack_file_name: String,
    mod_ids: HashSet<ModId>,
    overrides: Vec<String>, // MUST BE FULL PATH!!
    instance: Instance,
) {
    let index = ModIndex::load(&instance).await.unwrap();

    let mut urls: Vec<String> = Vec::new();
    let mut filenames: Vec<String> = Vec::new();
    let mut override_filenames: Vec<String> = Vec::new();

    for id in &mod_ids {
        let is_modrinth = matches!(id, ModId::Modrinth(_));

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

        if is_modrinth {
            urls.push(primary_file.url.clone());
            filenames.push(primary_file.filename.clone());
        } else {
            override_filenames.push(primary_file.filename.clone());
        }
    }

    let details = VersionDetails::load(&instance).await.unwrap();
    let minecraft_version = details.get_id();
    let config = ql_core::InstanceConfigJson::read(&instance).await;
    let loader_name = match config.as_ref().unwrap().mod_type.to_modrinth_str() {
        // Modrinth only supports these for modpacks
        "fabric" => "fabric-loader",
        "quilt" => "quilt-loader",
        "forge" => "forge",
        "neoforge" => "neoforge",
        _ => panic!("Unsupported loader type"),
    };
    let loader_version = config.unwrap().mod_type_info.unwrap().version;
    let mods_folder_path = instance.get_dot_minecraft_path().join("mods");
    let override_mods_full_path_string: Vec<String> =
        create_override_mods_full_path(override_filenames, &mods_folder_path);

    let full_path: Vec<PathBuf> = filenames
        .iter()
        .map(|rel_path| mods_folder_path.join(rel_path))
        .collect();

    let mut sha1s = Vec::new();
    let mut sha512s = Vec::new();
    let mut file_sizes = Vec::new();

    for path in &full_path {
        let hashes = hash_file(path).await.unwrap();
        sha1s.push(hashes.sha1);
        sha512s.push(hashes.sha512);
        file_sizes.push(hashes.file_size);
    }

    let json_data = create_modrinth_index_json(
        1,
        modpack_name,
        modpack_version,
        modpack_summary,
        loader_name.to_string(),
        loader_version.unwrap(),
        minecraft_version.to_string(),
        filenames
            .iter()
            .map(|name| format!("mods/{}", name))
            .collect::<Vec<String>>(),
        sha1s,
        sha512s,
        urls,
        file_sizes,
    )
    .unwrap();

    let zip_path = PathBuf::from(&modpack_path)
        .join(format!("{}.mrpack", modpack_file_name))
        .to_string_lossy()
        .to_string();

    let overrides: Vec<(String, String)> =
        overrides_fn(override_mods_full_path_string, overrides, instance);

    package_format_1_modpack("modrinth.index".to_string(), json_data, zip_path, overrides)
        .await
        .unwrap();
}

fn create_modrinth_index_json(
    format_version: u8,
    name: String,
    version_id: String,
    summary: String,
    loader_id: String,
    loader_version: String,
    minecraft_version: String,
    paths: Vec<String>,
    sha1: Vec<String>,
    sha512: Vec<String>,
    links: Vec<String>,
    file_size: Vec<u64>,
) -> std::io::Result<String> {
    let mut dependencies = Map::new();
    dependencies.insert("minecraft".to_string(), Value::String(minecraft_version));
    dependencies.insert(loader_id.to_string(), Value::String(loader_version));

    let sha1: Vec<&str> = sha1.iter().map(|s| s.as_str()).collect();
    let sha512: Vec<&str> = sha512.iter().map(|s| s.as_str()).collect();

    let files: Vec<FormatMQFileEntry> = paths
        .iter()
        .zip(&sha1)
        .zip(&sha512)
        .zip(&links)
        .zip(&file_size)
        .map(
            |((((path, sha1), sha512), download), &file_size)| FormatMQFileEntry {
                path: path.to_string(),
                hashes: Hashes {
                    sha1: sha1.to_string(),
                    sha512: sha512.to_string(),
                },
                downloads: vec![download.to_string()],
                file_size,
            },
        )
        .collect();

    let manifest = ModrinthModpackManifest {
        format_version,
        game: "minecraft".to_string(),
        version_id,
        name,
        summary,
        files,
        dependencies: Value::Object(dependencies),
    };

    let json_data = serde_json::to_string_pretty(&manifest)?;

    Ok(json_data)
}