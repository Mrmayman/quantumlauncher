use std::{
    collections::{HashMap, HashSet},
    sync::mpsc::Sender,
};

use ql_core::{
    GenericProgress, Instance, InstanceConfigJson, download, err, file_utils, info,
    json::VersionDetails, pt,
};

use crate::store::{
    CurseforgeNotAllowed, DirStructure, ModConfig, ModError, ModFile, ModId, ModIndex, QueryType,
    StoreBackendType,
    curseforge::{ModQuery, get_query_type},
    install_modpack,
};

use super::Mod;

pub struct ModDownloader<'a> {
    version: String,
    instance: Instance,
    loader: Option<&'static str>,
    pub index: ModIndex,

    dirs: DirStructure,

    pub query_cache: HashMap<String, Mod>,
    pub not_allowed: HashSet<CurseforgeNotAllowed>,
    already_installed: HashSet<String>,
    pub sender: Option<&'a Sender<GenericProgress>>,
}

impl<'a> ModDownloader<'a> {
    pub async fn new(
        instance: Instance,
        sender: Option<&'a Sender<GenericProgress>>,
    ) -> Result<Self, ModError> {
        let version_json = VersionDetails::load(&instance).await?;
        let config = InstanceConfigJson::read(&instance).await?;

        Ok(Self {
            version: version_json.get_id().to_owned(),
            loader: config.mod_type.not_vanilla().map(|n| n.to_curseforge_num()),
            index: ModIndex::load(&instance).await?,
            dirs: DirStructure::new(&instance, &version_json).await?,
            already_installed: HashSet::new(),
            query_cache: HashMap::new(),
            instance,
            sender,
            not_allowed: HashSet::new(),
        })
    }

    pub async fn basic(instance: Instance) -> Result<Self, ModError> {
        let version_json = VersionDetails::load(&instance).await?;
        let config = InstanceConfigJson::read(&instance).await?;

        Ok(Self {
            version: version_json.get_id().to_owned(),
            loader: config.mod_type.not_vanilla().map(|n| n.to_curseforge_num()),
            index: ModIndex::default(),
            dirs: DirStructure::new(&instance, &version_json).await?,
            already_installed: HashSet::new(),
            query_cache: HashMap::new(),
            instance,
            sender: None,
            not_allowed: HashSet::new(),
        })
    }

    pub async fn get_download_link(
        &mut self,
        id: &str,
        query_type: QueryType,
    ) -> Result<String, ModError> {
        let response = self.get_query(id).await?;

        let file_query = response
            .get_file(
                response.name.clone(),
                id,
                self.version.clone(),
                self.loader,
                query_type,
            )
            .await?;

        file_query.0.data.downloadUrl.ok_or(ModError::NoFilesFound)
    }

    pub async fn download(&mut self, id: &str, dependent: Option<&str>) -> Result<(), ModError> {
        // Mod already installed.
        if !self.already_installed.insert(id.to_owned()) {
            return Ok(());
        }
        if let Some(config) = self.index.mods.get_mut(&mid(id)) {
            // Is this mod a dependency of something else?
            if let Some(dependent) = dependent {
                config.dependents.insert(mid(dependent));
            } else {
                config.manually_installed = true;
            }
            return Ok(());
        }

        if let Some(dependent) = dependent {
            info!("Installing mod (id: {id}, dependency of {dependent})");
        } else {
            info!("Installing mod (id: {id})");
        }
        let response = self.get_query(id).await?;
        pt!("Name: {}", response.name);

        if let Some(config) = self
            .index
            .mods
            .values_mut()
            .find(|n| n.name == response.name)
        {
            pt!("Already installed from modrinth? Skipping...");
            // Is this mod a dependency of something else?
            if let Some(dependent) = dependent {
                config.dependents.insert(mid(dependent));
            } else {
                config.manually_installed = true;
            }
            return Ok(());
        }

        let query_type = get_query_type(response.class_id).await?;

        let (file_query, file_id) = response
            .get_file(
                response.name.clone(),
                id,
                self.version.clone(),
                self.loader,
                query_type,
            )
            .await?;
        let Some(url) = file_query.data.downloadUrl.clone() else {
            self.not_allowed.insert(CurseforgeNotAllowed {
                name: response.name.clone(),
                slug: response.slug.clone(),
                filename: file_query.data.fileName.clone(),
                project_type: query_type.to_curseforge_str().to_owned(),
                file_id: file_id as usize,
            });
            return Ok(());
        };

        let dir = match query_type {
            QueryType::DataPacks => &self.dirs.data_packs,
            QueryType::Mods => &self.dirs.mods,
            QueryType::ResourcePacks => &self.dirs.resource_packs,
            QueryType::Shaders => &self.dirs.shaders,
            QueryType::ModPacks => {
                let bytes = file_utils::download_file_to_bytes(&url, true).await?;
                self.index.save(&self.instance).await?;
                if let Some(not_allowed_new) =
                    install_modpack(bytes, self.instance.clone(), self.sender)
                        .await
                        .map_err(Box::new)?
                {
                    self.not_allowed.extend(not_allowed_new);
                } else {
                    err!("Invalid modpack downloaded from curseforge! Corrupted?");
                }
                self.index = ModIndex::load(&self.instance).await?;
                return Ok(());
            }
        };

        let file_dir = dir.join(&file_query.data.fileName);
        download(&url).user_agent_ql().path(&file_dir).await?;

        let id_str = response.id.to_string();
        let id_mod = ModId::Curseforge(id_str.clone());

        for dependency in &file_query.data.dependencies {
            let dep_id = dependency.modId.to_string();
            Box::pin(self.download(&dep_id, Some(id))).await?;
        }

        self.add_to_index(dependent, &response, query_type, file_query, url, &id_mod);

        pt!("Finished installing {query_type}: {}", response.name);

        Ok(())
    }

    pub async fn ensure_essential_mods(&mut self) -> Result<(), ModError> {
        const FABRIC: &str = "4";

        if self.loader == Some(FABRIC)
            && !self.index.mods.values_mut().any(|n| n.name == "Fabric API")
        {
            self.download("306612", None).await?;
        }
        Ok(())
    }

    fn add_to_index(
        &mut self,
        dependent: Option<&str>,
        response: &Mod,
        query_type: QueryType,
        file_query: super::CurseforgeFileQuery,
        url: String,
        id_mod: &ModId,
    ) {
        let QueryType::Mods = query_type else {
            return;
        };

        self.index.mods.insert(
            id_mod.clone(),
            ModConfig {
                name: response.name.clone(),
                manually_installed: dependent.is_none(),
                installed_version: file_query.data.displayName.clone(),
                version_release_time: file_query.data.fileDate.clone(),
                enabled: true,
                description: response.summary.clone(),
                icon_url: response.logo.clone().map(|n| n.url),
                project_source: StoreBackendType::Curseforge,
                project_id: id_mod.clone(),
                files: vec![ModFile {
                    url,
                    filename: file_query.data.fileName,
                    primary: true,
                }],
                supported_versions: file_query
                    .data
                    .gameVersions
                    .iter()
                    .filter(|n| n.contains('.'))
                    .cloned()
                    .collect(),
                dependencies: file_query
                    .data
                    .dependencies
                    .into_iter()
                    .map(|n| ModId::Curseforge(n.modId.to_string()))
                    .collect(),
                dependents: if let Some(dependent) = dependent {
                    let mut set = HashSet::new();
                    set.insert(mid(dependent));
                    set
                } else {
                    HashSet::new()
                },
            },
        );
    }

    async fn get_query(&mut self, id: &str) -> Result<Mod, ModError> {
        Ok(if let Some(r) = self.query_cache.get(id) {
            r.clone()
        } else {
            let query = ModQuery::load(id).await?;
            self.query_cache.insert(id.to_owned(), query.data.clone());
            query.data
        })
    }
}

fn mid(id: &str) -> ModId {
    ModId::Curseforge(id.to_owned())
}
