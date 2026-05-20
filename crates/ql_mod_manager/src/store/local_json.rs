use std::{
    collections::{HashMap, HashSet},
    io::ErrorKind,
    path::PathBuf,
    sync::Arc,
};

use ql_core::{
    Instance, IntoIoError, IntoJsonError, JsonFileError, file_utils::exists, info,
    json::VersionDetails,
};
use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::store::{DirStructure, ModId, QueryType};

use super::StoreBackendType;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ModConfig {
    pub name: Arc<str>,
    pub manually_installed: bool,
    pub installed_version: String,
    pub version_release_time: String,
    pub enabled: bool,
    pub description: String,
    pub icon_url: Option<String>,
    /// Source platform where the mod was downloaded from
    pub project_source: StoreBackendType,
    pub project_id: ModId,
    pub files: Vec<ModFile>,
    pub supported_versions: Vec<String>,
    pub dependencies: HashSet<ModId>,
    pub dependents: HashSet<ModId>,
    #[serde(default = "QueryType::default")]
    pub project_type: QueryType,
    /// If resource pack:
    /// - Whether in `resourcepacks` directory or `texturepacks` directory.
    ///
    /// More info may come for other project types in the future.
    pub project_type_extra: Option<String>,
    #[serde(flatten)] // Retain future fields for forward-compatibility
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ModIndex {
    pub mods: HashMap<ModId, ModConfig>,
    is_server: Option<bool>,
    #[serde(flatten)]
    _extra: HashMap<String, serde_json::Value>,
}

impl ModIndex {
    pub async fn load(selected_instance: &Instance) -> Result<Self, JsonFileError> {
        let mut index = load_inner(selected_instance).await?;
        index.fix(selected_instance.clone()).await?;
        Ok(index)
    }

    pub async fn save(&mut self, instance: &Instance) -> Result<(), JsonFileError> {
        let path = Self::get_path(instance);

        let index_str = serde_json::to_string(&self).json_to()?;
        fs::write(&path, &index_str).await.path(path)?;
        Ok(())
    }

    #[must_use]
    pub fn get_path(instance: &Instance) -> PathBuf {
        instance.get_dot_minecraft_path().join("mod_index.json")
    }

    fn new(instance: &Instance) -> Self {
        Self {
            mods: HashMap::new(),
            is_server: Some(instance.is_server()),
            _extra: HashMap::new(),
        }
    }

    async fn fix(&mut self, instance: Instance) -> Result<(), JsonFileError> {
        // I know this is inefficient, but modern OSes have automatic file caching
        // that basically guarantees this is fast.
        let details = VersionDetails::load(&instance).await?;

        let dirs = DirStructure::new(instance, &details).await?;

        self.fix_nonexistent_mods(&dirs).await;
        self.fix_cf_modpack_id_bug();

        Ok(())
    }

    fn fix_cf_modpack_id_bug(&mut self) {
        let mut drained_ids = Vec::new();
        for (k, v) in &self.mods {
            if let (StoreBackendType::Curseforge, ModId::Modrinth(_)) = (v.project_source, k) {
                // Curseforge modpack ID bug fix
                drained_ids.push(k.clone());
            }
        }
        let mut drained_mods = Vec::new();
        for id in drained_ids {
            if let Some(mod_cfg) = self.mods.remove(&id) {
                drained_mods.push((ModId::Curseforge(id.get_internal_id()), mod_cfg));
            }
        }
        self.mods.extend(drained_mods);
    }

    async fn fix_nonexistent_mods(&mut self, dirs: &DirStructure) {
        let mut removed_ids = Vec::new();
        let mut remove_dependents = Vec::new();

        for (id, mod_cfg) in &mut self.mods {
            let Some(content_dir) = dirs.get(mod_cfg.project_type) else {
                continue; // A modpack somehow ended up here, ignore to be safe
            };

            let mut removed = Vec::new();
            for (i, file) in mod_cfg.files.iter().enumerate() {
                let file_exists = if let QueryType::ResourcePacks = mod_cfg.project_type {
                    // I know this sucks but whatever
                    let p =
                        |base, name| dirs.instance.get_dot_minecraft_path().join(base).join(name);

                    let disabled_name = format!("{}.disabled", file.filename);

                    let enabled1_path = p("resourcepacks", &file.filename);
                    let disabled1_path = p("resourcepacks", &disabled_name);
                    let enabled2_path = p("texturepacks", &file.filename);
                    let disabled2_path = p("texturepacks", &disabled_name);

                    let (enabled1, disabled1, enabled2, disabled2) = tokio::join!(
                        exists(&enabled1_path),
                        exists(&disabled1_path),
                        exists(&enabled2_path),
                        exists(&disabled2_path)
                    );

                    enabled1 || disabled1 || enabled2 || disabled2
                } else {
                    let enabled_path = content_dir.join(&file.filename);
                    let disabled_path = content_dir.join(format!("{}.disabled", file.filename));
                    let (enabled, disabled) =
                        tokio::join!(exists(&enabled_path), exists(&disabled_path));

                    enabled || disabled
                };

                if !file_exists {
                    removed.push(i);
                }
            }
            for i in removed.into_iter().rev() {
                // Can't do Vec::retain because async
                mod_cfg.files.remove(i);
            }

            if mod_cfg.files.is_empty() {
                info!(
                    "Cleaning deleted {}: {}",
                    mod_cfg.project_type, mod_cfg.name
                );
                removed_ids.push(id.clone());

                for dependent in &mod_cfg.dependents {
                    remove_dependents.push((dependent.clone(), id.clone()));
                }
            }
        }

        for (dependent, dependency) in remove_dependents {
            if let Some(mod_cfg) = self.mods.get_mut(&dependent) {
                mod_cfg.dependencies.remove(&dependency);
            }
        }

        for id in removed_ids {
            self.mods.remove(&id);
        }
    }
}

async fn load_inner(selected_instance: &Instance) -> Result<ModIndex, JsonFileError> {
    let dot_mc_dir = selected_instance.get_dot_minecraft_path();

    let mods_dir = dot_mc_dir.join("mods");
    if !exists(&mods_dir).await {
        fs::create_dir(&mods_dir).await.path(&mods_dir)?;
    }

    let index_path = dot_mc_dir.join("mod_index.json");
    let old_index_path = mods_dir.join("index.json");

    // 1) Try migrating old index
    match fs::read_to_string(&old_index_path).await {
        Ok(index) if !index.trim().is_empty() => {
            let mod_index = serde_json::from_str(&index).json(index.clone())?;

            fs::write(&index_path, &index).await.path(index_path)?;
            fs::remove_file(&old_index_path)
                .await
                .path(old_index_path)?;

            return Ok(mod_index);
        }
        Ok(_) => {
            let _ = fs::remove_file(&old_index_path).await; // empty
        }
        Err(e) if e.kind() != ErrorKind::NotFound => {
            return Err(e.path(index_path).into());
        }
        _ => {}
    }

    // 2. Try current index
    match fs::read_to_string(&index_path).await {
        Ok(index) if !index.trim().is_empty() => {
            return Ok(serde_json::from_str::<ModIndex>(&index).json(index)?);
        }
        Ok(_) => {
            let _ = fs::remove_file(&old_index_path).await; // empty
        }
        Err(e) if e.kind() != ErrorKind::NotFound => {
            return Err(e.path(index_path).into());
        }
        _ => {}
    }

    let index = ModIndex::new(selected_instance);
    let index_str = serde_json::to_string(&index).json_to()?;

    let tmp = index_path.with_extension("json.tmp");
    fs::write(&tmp, &index_str).await.path(&tmp)?;
    fs::rename(&tmp, &index_path).await.path(&tmp)?;

    Ok(index)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModFile {
    // pub hashes: ModHashes,
    pub url: String,
    pub filename: String,
    pub primary: bool,
    // pub size: usize,
    // pub file_type: Option<String>,
}

// #[derive(Serialize, Deserialize, Debug, Clone)]
// pub struct ModHashes {
//     pub sha512: String,
//     pub sha1: String,
// }
