use std::{
    collections::{HashMap, HashSet},
    sync::mpsc::Sender,
};

use ql_core::{
    do_jobs, file_utils,
    json::{InstanceConfigJson, VersionDetails},
    pt, GenericProgress, InstanceSelection, IntoIoError,
};
use serde::Deserialize;
use tokio::sync::Mutex;

use crate::store::{
    curseforge::{self, get_query_type, CFSearchResult, CurseforgeFileQuery, ModQuery},
    get_dir, CurseforgeNotAllowed, ModConfig, ModFile, ModIndex, QueryType, SOURCE_ID_CURSEFORGE,
};

use super::PackError;

#[derive(Deserialize)]
pub struct PackIndex {
    pub minecraft: PackMinecraft,
    pub name: String,
    pub files: Vec<PackFile>,
    pub overrides: String,
}

#[derive(Deserialize)]
#[allow(non_snake_case)]
pub struct PackMinecraft {
    pub version: String,
    pub modLoaders: Vec<PackLoader>,
    // No one asked for your recommendation bro:
    // pub recommendedRam: usize
}

#[derive(Deserialize)]
pub struct PackLoader {
    pub id: String,
    // pub primary: bool,
}

#[derive(Deserialize)]
#[allow(non_snake_case)]
pub struct PackFile {
    pub projectID: usize,
    pub fileID: usize,
    pub required: bool,
}

impl PackFile {
    pub async fn download(
        &self,
        not_allowed: &Mutex<HashSet<CurseforgeNotAllowed>>,
        instance: &InstanceSelection,
        json: &VersionDetails,
        sender: Option<&Sender<GenericProgress>>,
        (i, len): (&Mutex<usize>, usize),
        cache: &HashMap<String, curseforge::Mod>,
        index: &Mutex<ModIndex>,
    ) -> Result<(), PackError> {
        if !self.required {
            return Ok(());
        }

        let project_id = self.projectID.to_string();
        let mod_info = if let Some(n) = cache.get(&project_id) {
            n.clone()
        } else {
            ModQuery::load(&project_id).await?.data
        };

        let query = CurseforgeFileQuery::load(&project_id, self.fileID as i32).await?;
        let query_type = get_query_type(mod_info.classId).await?;
        let Some(url) = query.data.downloadUrl.clone() else {
            self.add_to_not_allowed(not_allowed, mod_info, query, query_type)
                .await;
            return Ok(());
        };

        let path = get_dir(instance, json, query_type)
            .await?
            .join(&query.data.fileName);
        if path.is_file() {
            let metadata = tokio::fs::metadata(&path).await.path(&path)?;
            let got_len = metadata.len();
            if query.data.fileLength == got_len {
                pt!("Already installed {}, skipping", mod_info.name);
                return Ok(());
            }
        }

        file_utils::download_file_to_path(&url, true, &path).await?;
        add_to_index(index, project_id, &mod_info, query, url).await;

        send_progress(sender, i, len, &mod_info).await;
        Ok(())
    }

    async fn add_to_not_allowed(
        &self,
        not_allowed: &Mutex<HashSet<CurseforgeNotAllowed>>,
        mod_info: curseforge::Mod,
        query: CurseforgeFileQuery,
        query_type: QueryType,
    ) {
        not_allowed.lock().await.insert(CurseforgeNotAllowed {
            name: mod_info.name,
            slug: mod_info.slug,
            file_id: self.fileID,
            project_type: query_type.to_curseforge_str().to_owned(),
            filename: query.data.fileName,
        });
    }
}

async fn add_to_index(
    index: &Mutex<ModIndex>,
    project_id: String,
    mod_info: &curseforge::Mod,
    query: CurseforgeFileQuery,
    url: String,
) {
    let mut index = index.lock().await;
    if !index.mods.contains_key(&project_id) {
        index.mods.insert(
            project_id.clone(),
            ModConfig {
                name: mod_info.name.clone(),
                manually_installed: true,
                installed_version: query.data.displayName.clone(),
                version_release_time: query.data.fileDate.clone(),
                enabled: true,
                description: mod_info.summary.clone(),
                icon_url: mod_info.logo.clone().map(|n| n.url),
                project_source: SOURCE_ID_CURSEFORGE.to_owned(),
                project_id,
                files: vec![ModFile {
                    url,
                    filename: query.data.fileName,
                    primary: true,
                }],
                supported_versions: query
                    .data
                    .gameVersions
                    .iter()
                    .filter(|n| n.contains('.'))
                    .cloned()
                    .collect(),
                dependencies: HashSet::new(),
                dependents: HashSet::new(),
            },
        );
    }
}

async fn send_progress(
    sender: Option<&Sender<GenericProgress>>,
    i: &Mutex<usize>,
    len: usize,
    mod_info: &curseforge::Mod,
) {
    if let Some(sender) = sender {
        let mut i = i.lock().await;
        _ = sender.send(GenericProgress {
            done: *i,
            total: len,
            message: Some(format!(
                "Modpack: Installed mod (curseforge) ({i}/{len}):\n{}",
                mod_info.name,
                i = *i + 1,
            )),
            has_finished: false,
        });
        pt!(
            "Installed mod (curseforge) ({i}/{len}): {}",
            mod_info.name,
            i = *i + 1,
        );
        *i += 1;
    }
}

pub async fn install(
    instance: &InstanceSelection,
    config: &InstanceConfigJson,
    json: &VersionDetails,
    index: &PackIndex,
    sender: Option<&Sender<GenericProgress>>,
) -> Result<HashSet<CurseforgeNotAllowed>, PackError> {
    if json.get_id() != index.minecraft.version {
        return Err(PackError::GameVersion {
            expect: index.minecraft.version.clone(),
            got: json.get_id().to_owned(),
        });
    }

    pt!("CurseForge Modpack: {}", index.name);

    let loader = match config.mod_type.as_str() {
        "Forge" => "forge",
        "Fabric" => "fabric",
        "Quilt" => "quilt",
        "NeoForge" => "neoforge",
        _ => {
            return Err(expect_got_curseforge(index, config));
        }
    };

    if !index
        .minecraft
        .modLoaders
        .iter()
        .any(|n| n.id.starts_with(loader))
    {
        return Err(expect_got_curseforge(index, config));
    }

    let not_allowed = Mutex::new(HashSet::new());
    let len = index.files.len();

    let i = Mutex::new(0);
    let mod_index = Mutex::new(ModIndex::load(instance).await?);

    let cache: HashMap<String, curseforge::Mod> = {
        let project_ids: Vec<String> = index
            .files
            .iter()
            .map(|n| n.projectID.to_string())
            .collect();
        CFSearchResult::get_from_ids(&project_ids)
            .await?
            .data
            .into_iter()
            .map(|n| (n.id.to_string(), n))
            .collect()
    };

    do_jobs::<(), PackError>(index.files.iter().map(|file| {
        file.download(
            &not_allowed,
            instance,
            json,
            sender,
            (&i, len),
            &cache,
            &mod_index,
        )
    }))
    .await?;

    mod_index.lock().await.save(instance).await?;

    let not_allowed = not_allowed.lock().await;
    Ok(not_allowed.clone())
}

fn expect_got_curseforge(index: &PackIndex, config: &InstanceConfigJson) -> PackError {
    PackError::Loader {
        expect: index
            .minecraft
            .modLoaders
            .iter()
            .map(|l| l.id.split('-').next().unwrap_or(&l.id))
            .collect::<Vec<&str>>()
            .join(", "),
        got: config.mod_type.clone(),
    }
}
