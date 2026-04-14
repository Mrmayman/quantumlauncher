use std::{
    collections::{HashMap, HashSet},
    io::ErrorKind,
    path::Path,
};

use ql_core::{
    Instance, IntoIoError, IntoJsonError, IoError, JsonFileError, file_utils::exists, info,
};
use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::store::ModId;

use super::StoreBackendType;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ModConfig {
    pub name: String,
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
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ModIndex {
    pub mods: HashMap<ModId, ModConfig>,
    pub is_server: Option<bool>,
}

impl ModIndex {
    pub async fn load(selected_instance: &Instance) -> Result<Self, JsonFileError> {
        let mut index = load_inner(selected_instance).await?;
        index.fix(selected_instance).await?;
        Ok(index)
    }

    pub async fn save(
        &mut self,
        selected_instance: &Instance,
    ) -> Result<(), JsonFileError> {
        let index_dir = selected_instance
            .get_dot_minecraft_path()
            .join("mod_index.json");

        let index_str = serde_json::to_string(&self).json_to()?;
        fs::write(&index_dir, &index_str).await.path(index_dir)?;
        Ok(())
    }

    fn new(instance_name: &Instance) -> Self {
        Self {
            mods: HashMap::new(),
            is_server: Some(instance_name.is_server()),
        }
    }

    pub async fn fix(&mut self, selected_instance: &Instance) -> Result<(), IoError> {
        let mods_dir = selected_instance.get_dot_minecraft_path().join("mods");
        if !exists(&mods_dir).await {
            fs::create_dir(&mods_dir).await.path(&mods_dir)?;
            self.mods.clear();
            return Ok(());
        }

        self.fix_nonexistent_mods(&mods_dir);
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
                drained_mods.push((ModId::Curseforge(id.get_internal_id().to_owned()), mod_cfg));
            }
        }
        self.mods.extend(drained_mods);
    }

    fn fix_nonexistent_mods(&mut self, mods_dir: &Path) {
        let mut removed_ids = Vec::new();
        let mut remove_dependents = Vec::new();

        for (id, mod_cfg) in &mut self.mods {
            mod_cfg.files.retain(|file| {
                mods_dir.join(&file.filename).is_file()
                    || mods_dir
                        .join(format!("{}.disabled", file.filename))
                        .is_file()
            });
            if mod_cfg.files.is_empty() {
                info!("Cleaning deleted mod: {}", mod_cfg.name);
                removed_ids.push(id.clone());
            }
            for dependent in &mod_cfg.dependents {
                remove_dependents.push((dependent.clone(), id.clone()));
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
        Ok(index) => {
            let mod_index = serde_json::from_str(&index).json(index.clone())?;

            fs::write(&index_path, &index).await.path(index_path)?;
            fs::remove_file(&old_index_path)
                .await
                .path(old_index_path)?;

            return Ok(mod_index);
        }
        Err(e) if e.kind() != ErrorKind::NotFound => {
            return Err(e.path(index_path).into());
        }
        _ => {}
    }

    // 2. Try current index
    match fs::read_to_string(&index_path).await {
        Ok(index) => return Ok(serde_json::from_str::<ModIndex>(&index).json(index)?),
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
