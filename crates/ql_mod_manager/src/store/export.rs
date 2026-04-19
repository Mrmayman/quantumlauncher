use crate::store::{ModId, ModIndex};
use async_zip::tokio::write::ZipFileWriter;
use async_zip::{Compression, ZipEntryBuilder};
use hex;
use ql_core::InstanceSelection;
use ql_core::json::VersionDetails;
use serde::Serialize;
use serde_json::{Map, Value};
use sha1::{Digest, Sha1};
use sha2::Sha512;
use std::collections::HashSet;
use std::fs;
use std::io::Result as StdResult;
use std::path::{Path, PathBuf};
use tokio::fs::read;

#[derive(Serialize)]
pub struct FormatMQFileEntry {
    // This file entry is used for Modrinth and QLMP
    path: String,
    hashes: Hashes,
    #[serde(rename = "downloads")]
    downloads: Vec<String>,
    #[serde(rename = "fileSize")]
    file_size: u64,
}

#[derive(Serialize)]
pub struct Hashes {
    sha1: String,
    sha512: String,
}

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
pub struct QlModpackManifest {
    format_version: u8,
    minecraft_version: String,
    loader_id: Value,
    version_id: String,
    name: String,
    author: String,
    summary: String,
    icon: String,
    files: Vec<FormatMQFileEntry>,
}

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

pub async fn export_modrinth_modpack(
    modpack_path: String,
    modpack_name: String,
    modpack_version: String,
    modpack_summary: String,
    modpack_file_name: String,
    mod_ids: HashSet<ModId>,
    overrides: Vec<String>, // MUST BE FULL PATH!!
    instance: InstanceSelection,
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
    let loader_name = match config.unwrap().mod_type.to_modrinth_str() {
        // Modrinth only supports these for modpacks
        "fabric" => {"fabric-loader"},
        "quilt" => {"quilt-loader"},
        "forge" => {"forge"},
        "neofroge" => {"neoforge"},
        _ => panic!("Unsupported loader type"),
    };

    let config = ql_core::InstanceConfigJson::read(&instance).await;
    let loader_version = config.unwrap().mod_type_info.unwrap().version;

    let mods_folder_path = instance.get_dot_minecraft_path().join("mods");

    let override_mods_full_path_string: Vec<String> = override_filenames
        .iter()
        .map(|rel_path| mods_folder_path.join(rel_path))
        .collect::<Vec<_>>()
        .into_iter()
        .map(|path| path.into_os_string().into_string().unwrap())
        .collect();

    let full_path: Vec<PathBuf> = filenames
        .iter()
        .map(|rel_path| mods_folder_path.join(rel_path))
        .collect();

    let file_sizes: Vec<u64> = full_path
        .iter()
        .map(|path| fs::metadata(path).map(|meta| meta.len()).unwrap_or(0))
        .collect();

    let sha1s: Vec<String> = full_path
        .clone()
        .into_iter()
        .map(|path| {
            let data = fs::read(path).unwrap();
            let mut hasher = Sha1::new();
            hasher.update(&data);
            let hash = hasher.finalize();
            hex::encode(hash)
        })
        .collect();

    let sha512s: Vec<String> = full_path
        .into_iter()
        .map(|path| {
            let data = fs::read(path).unwrap();
            let mut hasher = Sha512::new();
            hasher.update(&data);
            let hash = hasher.finalize();
            hex::encode(hash)
        })
        .collect();

    let json_data = create_modrinth_index_json(
        1,
        modpack_name,
        modpack_version,
        modpack_summary,
        loader_name.to_string(),
        loader_version.unwrap(),
        minecraft_version.to_string(),
        filenames.iter()
            .map(|name| format!("mods/{}", name))
            .collect::<Vec<String>>(),
        sha1s,
        sha512s,
        urls,
        file_sizes,
    )
    .unwrap();

    let zip_path = modpack_path + "/" + modpack_file_name.as_str() + ".mrpack";

    let overrides: Vec<(String, String)> = overrides
        .into_iter()
        .chain(override_mods_full_path_string)
        .into_iter()
        .collect::<Vec<String>>()
        .into_iter()
        .map(|full| {
            let path = Path::new(&full);
            let relative = path
                .strip_prefix(Path::new(
                    &instance.get_dot_minecraft_path().to_str().unwrap(),
                ))
                .unwrap_or(path);
            (full.clone(), relative.to_string_lossy().into())
        })
        .collect();


    package_format1_pack("modrinth.index".to_string(), json_data, zip_path, overrides).unwrap();
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
) -> StdResult<String> {
    let mut dependencies = Map::new();
    dependencies.insert("minecraft".to_string(), Value::String(minecraft_version));
    dependencies.insert(loader_id.to_string(), Value::String(loader_version));

    let files: Vec<FormatMQFileEntry> = format_1_file_entry(paths, sha1, sha512, links, file_size)?;

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

pub async fn export_curseforge_modpack(
    author: String,
    modpack_name: String,
    modpack_version: String,
    modpack_file_name: String,
    mod_ids: HashSet<ModId>,
    overrides: Vec<String>, // MUST BE FULL PATH!!
    icon_url: String,
    instance: InstanceSelection) {

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
            continue
        } else {
            override_filenames.push(primary_file.filename.clone());
        }
    }

    let details = VersionDetails::load(&instance).await.unwrap();
    let minecraft_version = details.get_id().to_string();
    let config = ql_core::InstanceConfigJson::read(&instance).await;
    let loader_name = match config.unwrap().mod_type.to_curseforge_num() {
        "1" => {"forge"},
        "4" => {"fabric"},
        "5" => {"quilt"},
        "6" => {"neoforge"},
        "3" => {"lightloader"},
        _ => panic!()
    };

    let config = ql_core::InstanceConfigJson::read(&instance).await;
    let loader_version = config.unwrap().mod_type_info.unwrap().version;
    let loader = loader_name.to_string() + "-" + loader_version.unwrap().as_str();

    let file_ids: Vec<&str> = vec!["temp"]; // TODO: get FileIds here!!

    let mod_ids: Vec<String>= mod_ids
        .into_iter()
        .map(|map| serde_json::to_string(&map).unwrap())
        .collect();

    let mods_folder_path = instance.get_dot_minecraft_path().join("mods");

    let override_mods_full_path_string: Vec<String> = override_filenames
        .iter()
        .map(|rel_path| mods_folder_path.join(rel_path))
        .collect::<Vec<_>>()
        .into_iter()
        .map(|path| path.into_os_string().into_string().unwrap())
        .collect();

    let overrides: Vec<(String, String)> = overrides
        .into_iter()
        .chain(override_mods_full_path_string)
        .into_iter()
        .collect::<Vec<String>>()
        .into_iter()
        .map(|full| {
            let path = Path::new(&full);
            let relative = path
                .strip_prefix(Path::new(
                    &instance.get_dot_minecraft_path().to_str().unwrap(),
                ))
                .unwrap_or(path);
            (full.clone(), relative.to_string_lossy().into())
        })
        .collect();

    let json_data = write_curseforge_manifest_json(
        mod_ids,
        file_ids,
        author,
        modpack_version,
        modpack_name,
        loader,
        minecraft_version,
        icon_url
    )
    .unwrap();

    package_format1_pack("manifest".to_string(), json_data, modpack_file_name, overrides).unwrap();

}

fn write_curseforge_manifest_json(
    mod_id: Vec<String>,
    file_id: Vec<&str>,
    author: String,
    modpack_version: String,
    name: String,
    loader_id: String,
    version: String,
    image: String
) -> StdResult<String> {

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
            mod_loaders: vec![CurseForgeModLoader { id: loader_id, primary }],
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

/*
pub async fn export_qlmp_modpack(author: String, icon: String, modpack_path: String, modpack_name: String,modpack_version: String, modpack_summary: String,modpack_file_name: String, mod_ids: HashSet<ModId>, overrides_full_path: Vec<String>, instance: InstanceSelection)  {

    let mut urls: Vec<String> = Vec::new();
    let mut filenames: Vec<Format1FileEntry> = Vec::new();


    let details = VersionDetails::load(&instance).await.unwrap();
    let minecraft_version = details.get_id();
    let config = ql_core::InstanceConfigJson::read(&instance).await;
    let loader_name = config.unwrap().mod_type.to_modrinth_str();  // TODO: INCORRECT: Waiting for change
    let config = ql_core::InstanceConfigJson::read(&instance).await;
    let loader_version = config.unwrap().mod_type_info.unwrap().version;

    let minecraft_path = instance.get_dot_minecraft_path();

    let paths: Vec<String> = filenames
        .iter()
        .map(|name| format!("mods/{}", name))
        .collect();

    let full_path: Vec<PathBuf> = paths
        .iter()
        .map(|rel_path| minecraft_path.join(rel_path))
        .collect();


    let file_sizes: Vec<u64> = full_path
        .iter()
        .map(|path| fs::metadata(path).map(|meta| meta.len()).unwrap_or(0))
        .collect();


    let sha1s: Vec<String> = full_path
        .clone()
        .into_iter()
        .map(|path| {
            let data = fs::read(path).unwrap();
            let mut hasher = Sha1::new();
            hasher.update(&data);
            let hash = hasher.finalize();
            hex::encode(hash)
        })
        .collect();


    let sha512s: Vec<String> = full_path
        .into_iter()
        .map(|path| {
            let data = fs::read(path).unwrap();
            let mut hasher = Sha512::new();
            hasher.update(&data);
            let hash = hasher.finalize();
            hex::encode(hash)
        })
        .collect();

    let json_data = create_qlmp_index_json(1, minecraft_version, loader_name, loader_version, modpack_version, modpack_name, author, modpack_summary, icon, paths, sha1s, sha512s, urls, file_sizes).unwrap();

    let zip_path= modpack_path + "/" + modpack_file_name.as_str() + ".qlmp";

    let result: Vec<(String, String)> = overrides_full_path
        .iter()
        .map(|full| {
            let path = std::path::Path::new(full);
            let relative = path.strip_prefix(std::path::Path::new(&instance.get_dot_minecraft_path().to_str().unwrap())).unwrap_or(path);
            (full.clone(), relative.to_string_lossy().into())
        })
        .collect();

    let overrides = result.clone();

    package_format1_pack(json_data, zip_path, overrides).unwrap();
}
 */

fn create_qlmp_index_json(
    format_version: u8,
    minecraft_version: String,
    loader_id: String,
    loader_version: String,
    version_id: String,
    name: String,
    author: String,
    summary: String,
    icon: String,
    paths: Vec<String>,
    sha1: Vec<String>,
    sha512: Vec<String>,
    links: Vec<String>,
    file_size: Vec<u64>,
) -> StdResult<String> {
    let mut loader = Map::new();
    loader.insert(loader_id.to_string(), Value::String(loader_version));

    let files: Vec<FormatMQFileEntry> = format_1_file_entry(paths, sha1, sha512, links, file_size)?;

    let manifest = QlModpackManifest {
        format_version,
        minecraft_version,
        loader_id: Value::Object(loader),
        version_id,
        name,
        author,
        summary,
        icon,
        files,
    };

    let json_data = serde_json::to_string_pretty(&manifest)?;

    Ok(json_data)
}

#[derive(thiserror::Error, Debug)]
enum PackageError {
    #[error("zip error: {0}")]
    Zip(#[from] async_zip::error::ZipError),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

#[tokio::main]
async fn package_format1_pack(
    //  Used for Modrinth, QLMP and CurseForge packs
    json_name: String,
    json_data: String,
    zip_path: String,
    overrides: Vec<(String, String)>,
) -> Result<(), PackageError> {
    let parent_dir = Path::new(&zip_path).parent().unwrap();
    tokio::fs::create_dir_all(parent_dir).await?;

    let output_file = tokio::fs::File::create(&zip_path).await?;
    let mut writer = ZipFileWriter::with_tokio(output_file);

    for (full_path, relative_path) in &overrides {
        let in_zip_path = format!("overrides/{}", relative_path);
        add_file_to_zip(&mut writer, full_path, &in_zip_path).await?;
    }

    let json_builder = ZipEntryBuilder::new(json_name.into(), Compression::Deflate);
    writer
        .write_entry_whole(json_builder, json_data.as_bytes())
        .await?;

    writer.close().await?;
    Ok(())
}

async fn add_file_to_zip<W: tokio::io::AsyncWrite + Unpin>(
    writer: &mut ZipFileWriter<W>,
    original_file_path: &str,
    zip_relative_path: &str,
) -> StdResult<()> {
    let data = read(original_file_path).await?;
    let builder = ZipEntryBuilder::new(zip_relative_path.into(), Compression::Deflate);
    writer.write_entry_whole(builder, &data).await.unwrap();
    Ok(())
}

fn format_1_file_entry(
    // Used by Modrinth and QLMP
    paths: Vec<String>,
    sha1: Vec<String>,
    sha512: Vec<String>,
    links: Vec<String>,
    file_size: Vec<u64>,
) -> StdResult<Vec<FormatMQFileEntry>> {
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

    Ok(files)
}