use async_zip::tokio::write::ZipFileWriter;
use async_zip::{Compression, ZipEntryBuilder};
use hex;
use ql_core::{Instance, JsonFileError};
use serde::Serialize;
use sha1::{Digest, Sha1};
use sha2::Sha512;
use std::io::Result as StdResult;
use std::path::Path;
use thiserror::Error;
use tokio::fs::read;

mod curseforge;
mod modrinth;
mod multimc;

#[derive(Serialize)]
pub struct Hashes {
    sha1: String,
    sha512: String,
}

struct FileHashes {
    sha1: String,
    sha512: String,
    file_size: u64,
}

#[derive(Error, Debug)]
pub enum ModpackExportError {
    #[error("zip error: {0}")]
    Zip(#[from] async_zip::error::ZipError),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("failed to parse JSON: {0}")]
    Json(#[from] ql_core::JsonFileError),

    #[error("manifest serialization failed: {0}")]
    ManifestSerialization(#[from] serde_json::Error),
}

#[derive(Error, Debug)]
pub enum PackageError {
    #[error("zip error: {0}")]
    Zip(#[from] async_zip::error::ZipError),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("parent path is undefined for: {0}")]
    #[allow(unused)]
    ParentPathUndefined(String),
}

async fn package_format_1_modpack(
    //  Used for Modrinth and CurseForge packs
    json_name: String,
    json_data: String,
    zip_path: String,
    overrides: Vec<(String, String)>,
) -> Result<(), PackageError> {
    let parent_dir = Path::new(&zip_path)
        .parent()
        .ok_or(PackageError::ParentPathUndefined(zip_path.clone()))?;
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

async fn package_format_2_modpack() {


}

async fn add_file_to_zip<W: tokio::io::AsyncWrite + Unpin>(
    writer: &mut ZipFileWriter<W>,
    original_file_path: &str,
    zip_relative_path: &str,
) -> Result<(), PackageError> {
    let data = read(original_file_path).await?;
    let builder = ZipEntryBuilder::new(zip_relative_path.into(), Compression::Deflate);
    writer.write_entry_whole(builder, &data).await?;
    Ok(())
}

fn overrides_fn(
    override_mods_full_path_string: Vec<String>,
    overrides: Vec<String>,
    instance: Instance,
) -> Vec<(String, String)> {
    let overrides: Vec<(String, String)> = overrides
        .into_iter()
        .chain(override_mods_full_path_string)
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

    overrides
}

async fn hash_file(path: &Path) -> StdResult<FileHashes> {
    let data = tokio::fs::read(path).await?;

    let mut sha1 = Sha1::new();
    let mut sha512 = Sha512::new();
    sha1.update(&data);
    sha512.update(&data);

    Ok(FileHashes {
        sha1: hex::encode(sha1.finalize()),
        sha512: hex::encode(sha512.finalize()),
        file_size: data.len() as u64,
    })
}

fn create_override_mods_full_path(
    override_filenames: Vec<String>,
    mods_folder_path: &Path,
) -> Vec<String> {
    let override_mods_full_path_string: Vec<String> = override_filenames
        .iter()
        .map(|rel_path| mods_folder_path.join(rel_path))
        .map(|path| path.into_os_string().to_string_lossy().to_string())
        .collect();

    override_mods_full_path_string
}