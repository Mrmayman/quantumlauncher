use std::{collections::HashMap, path::Path, sync::mpsc::Sender};

use ql_core::{
    GenericProgress, Instance, InstanceKind, Loader, do_jobs, download,
    json::{InstanceConfigJson, VersionDetails},
    pt,
};
use serde::Deserialize;
use tokio::sync::Mutex;

use super::PackError;

#[derive(Deserialize)]
pub struct PackIndex {
    pub name: String,
    pub files: Vec<PackFile>,

    /// Info about which Minecraft version
    /// and Loader version is required. May contain:
    ///
    /// - `minecraft` (always present)
    /// - `forge`
    /// - `neoforge`
    /// - `fabric-loader`
    /// - `quilt-loader`
    pub dependencies: HashMap<String, String>,
}

#[derive(Deserialize)]
pub struct PackFile {
    pub path: String,
    pub env: PackEnv,
    pub downloads: Vec<String>,
}

#[derive(Deserialize)]
pub struct PackEnv {
    pub client: String,
    pub server: String,
}

pub async fn install(
    instance: &Instance,
    mc_dir: &Path,
    config: &InstanceConfigJson,
    json: &VersionDetails,
    index: &PackIndex,
    sender: Option<&Sender<GenericProgress>>,
) -> Result<(), PackError> {
    if let Some(version) = index.dependencies.get("minecraft") {
        if json.get_id() != *version {
            return Err(PackError::GameVersion {
                expect: version.clone(),
                got: json.get_id().to_owned(),
            });
        }
    }

    pt!("Modrinth Modpack: {}", index.name);
    let loader = match config.mod_type {
        Loader::Forge => "forge",
        Loader::Fabric => "fabric-loader",
        Loader::Quilt => "quilt-loader",
        Loader::Neoforge => "neoforge",
        _ => {
            return Err(expect_got_modrinth(index, config));
        }
    };
    if !index.dependencies.contains_key(loader) {
        return Err(expect_got_modrinth(index, config));
    }

    let i = Mutex::new(0);
    let i = &i;

    let len = index.files.len();
    let jobs: Result<Vec<()>, PackError> = do_jobs(
        index
            .files
            .iter()
            .filter_map(|file| file.downloads.first().map(|n| (file, n)))
            .map(|(file, url)| async move {
                let required_field = match instance.kind {
                    InstanceKind::Client => &file.env.client,
                    InstanceKind::Server => &file.env.server,
                };
                if required_field != "required" {
                    pt!("Skipping {} (optional)", file.path);
                    return Ok(());
                }

                // Known broken mods, included in Re-Console modpack
                // https://modrinth.com/modpack/legacy-minecraft
                // These fix the crash, but I still get a black screen
                let url = if url == "https://cdn.modrinth.com/data/u58R1TMW/versions/WFiIDhbD/connector-2.0.0-beta.2%2B1.21.1-full.jar" {
                    "https://cdn.modrinth.com/data/u58R1TMW/versions/k3UrqfQk/connector-2.0.0-beta.6%2B1.21.1-full.jar"
                } else if url == "https://cdn.modrinth.com/data/gHvKJofA/versions/GvTZJhPo/Legacy4J-1.21-1.7.2-neoforge.jar"
                    || url == "https://cdn.modrinth.com/data/gHvKJofA/versions/fYlGcfZd/Legacy4J-1.21-1.7.3-neoforge.jar" {
                    "https://cdn.modrinth.com/data/gHvKJofA/versions/RD8XgI0Y/Legacy4J-1.21-1.7.4-neoforge.jar"
                } else {
                    url
                };

                let bytes_path = mc_dir.join(&file.path);
                download(url).user_agent_ql().path(&bytes_path).await?;

                send_progress(sender, i, len, file).await;

                Ok(())
            }),
    )
    .await;
    jobs?;

    Ok(())
}

async fn send_progress(
    sender: Option<&Sender<GenericProgress>>,
    i: &Mutex<usize>,
    len: usize,
    file: &PackFile,
) {
    if let Some(sender) = sender {
        let mut i = i.lock().await;
        _ = sender.send(GenericProgress {
            done: *i,
            total: len,
            message: Some(format!(
                "Modpack: Installed mod (modrinth) ({i}/{len}):\n{}",
                file.path,
                i = *i + 1
            )),
            has_finished: false,
        });
        pt!(
            "Installed mod (modrinth) ({i}/{len}): {}",
            file.path,
            i = *i + 1,
        );
        *i += 1;
    }
}

fn expect_got_modrinth(index_json: &PackIndex, config: &InstanceConfigJson) -> PackError {
    match index_json
        .dependencies
        .keys()
        .filter_map(|k| (k != "minecraft").then_some(k.clone()))
        .map(|loader| {
            loader
                .strip_suffix("-loader")
                .map(str::to_owned)
                .unwrap_or(loader)
        })
        .next()
    {
        Some(expect) => PackError::Loader {
            expect,
            got: config.mod_type,
        },
        None => PackError::NoLoadersSpecified,
    }
}
