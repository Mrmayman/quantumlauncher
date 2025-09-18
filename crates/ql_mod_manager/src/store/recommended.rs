use std::sync::{mpsc::Sender, Arc, Mutex};

use futures::StreamExt;
use owo_colors::colored::OwoColorize;
use ql_core::{
    err_no_log, info, info_no_log, json::VersionDetails, pt, GenericProgress, InstanceSelection,
    Loader, ModId, StoreBackendType,
};

use crate::store::{get_latest_version_date, ModIndex};

use super::ModError;

#[derive(Debug, Clone)]
pub struct RecommendedMod {
    pub id: &'static str,
    pub name: &'static str,
    pub backend: StoreBackendType,
    pub description: &'static str,
    pub enabled_by_default: bool,
}

impl RecommendedMod {
    pub async fn get_compatible_mods(
        ids: Vec<Self>,
        instance: InstanceSelection,
        loader: Loader,
        sender: Sender<GenericProgress>,
    ) -> Result<Vec<Self>, ModError> {
        const LIMIT: usize = 128;

        let json = VersionDetails::load(&instance).await?;
        let index = ModIndex::load(&instance).await?;
        let version = json.get_id();

        info!("Checking compatibility");
        let mut mods = Vec::new();
        let len = ids.len();

        let i = Arc::new(Mutex::new(0));

        let mut tasks = futures::stream::FuturesOrdered::new();
        for id in ids {
            let i = i.clone();
            tasks.push_back(id.check_compatibility(&sender, i, len, loader, version, &index));
            if tasks.len() > LIMIT {
                if let Some(task) = tasks.next().await.flatten() {
                    mods.push(task);
                }
            }
        }

        while let Some(task) = tasks.next().await {
            if let Some(task) = task {
                mods.push(task);
            }
        }

        Ok(mods)
    }

    async fn check_compatibility(
        self,
        sender: &Sender<GenericProgress>,
        i: Arc<Mutex<usize>>,
        len: usize,
        loader: Loader,
        version: &str,
        index: &ModIndex,
    ) -> Option<Self> {
        let mod_id = ModId::from_pair(self.id, self.backend);
        if index.mods.contains_key(&mod_id.get_index_str())
            || index.mods.iter().any(|n| n.1.name == self.name)
        {
            return None;
        }

        let is_compatible = get_latest_version_date(Some(loader), &mod_id, version).await;
        let is_compatible = match is_compatible {
            Ok(_) => {
                pt!("{} compatible!", self.name);
                true
            }
            Err(ModError::NoCompatibleVersionFound(_)) => {
                pt!("{} {}", self.name, "not compatible!".bright_black());
                false
            }
            Err(ModError::RequestError(err)) => {
                err_no_log!("{}", err.summary());
                false
            }
            Err(err) => {
                err_no_log!("{err}");
                false
            }
        };

        {
            let mut i = i.lock().unwrap();
            *i += 1;
            if sender
                .send(GenericProgress {
                    done: *i,
                    total: len,
                    message: Some(format!("Checked compatibility: {}", self.name)),
                    has_finished: false,
                })
                .is_err()
            {
                info_no_log!("Cancelled recommended mod check");
                return None;
            }
        }

        is_compatible.then_some(self)
    }
}

pub const RECOMMENDED_MODS: &[RecommendedMod] = &[
    RecommendedMod {
        id: "AANobbMI",
        name: "Sodium",
        description: "Optimizes the rendering engine",
        enabled_by_default: true,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "gvQqBUqZ",
        name: "Lithium",
        description: "Optimizes the integrated server",
        enabled_by_default: true,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "mOgUt4GM",
        name: "Mod Menu",
        description: "A mod menu for managing mods",
        enabled_by_default: true,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "NNAgCjsB",
        name: "Entity Culling",
        description: "Optimizes entity rendering",
        enabled_by_default: true,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "5ZwdcRci",
        name: "ImmediatelyFast",
        description: "Optimizes immediate mode rendering",
        enabled_by_default: true,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "qQyHxfxd",
        name: "No Chat Reports",
        description: "Disables chat reporting",
        enabled_by_default: true,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "kzwxhsjp",
        name: "Accurate Block Placement Reborn",
        description: "Makes placing blocks more accurate",
        enabled_by_default: true,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "aC3cM3Vq",
        name: "Mouse Tweaks",
        description: "Improves inventory controls",
        enabled_by_default: true,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "hvFnDODi",
        name: "LazyDFU",
        description: "Speeds up Minecraft start time",
        enabled_by_default: true,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "YL57xq9U",
        name: "Iris Shaders",
        description: "Adds Shaders to Minecraft",
        enabled_by_default: false,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "1IjD5062",
        name: "Continuity",
        description: "Adds connected textures",
        enabled_by_default: false,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "yBW8D80W",
        name: "LambDynamicLights",
        description: "Adds dynamic lights",
        enabled_by_default: false,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "bXX9h73M",
        name: "MidnightControls",
        description: "Adds controller (and touch) support",
        enabled_by_default: false,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "8shC1gFX",
        name: "BetterF3",
        description: "Cleans up the debug (F3) screen",
        enabled_by_default: false,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "EsAfCjCV",
        name: "AppleSkin",
        description: "Shows hunger and saturation values",
        enabled_by_default: false,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "1bokaNcj",
        name: "Xaero's Minimap",
        description: "Adds a minimap to the game",
        enabled_by_default: false,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "NcUtCpym",
        name: "Xaero's World Map",
        description: "Adds a world map to the game",
        enabled_by_default: false,
        backend: StoreBackendType::Modrinth,
    },
];
