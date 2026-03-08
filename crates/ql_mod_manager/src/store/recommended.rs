use std::sync::{mpsc::Sender, Arc, Mutex};

use futures::StreamExt;
use owo_colors::colored::OwoColorize;
use ql_core::{
    err, info, json::VersionDetails, pt, GenericProgress, InstanceSelection, Loader, ModId,
    StoreBackendType,
};

use crate::store::{get_latest_version_date, ModIndex};

use super::ModError;

#[derive(Debug, Clone)]
pub struct RecommendedMod {
    pub id: &'static str,
    pub name: &'static str,
    pub backend: StoreBackendType,
    pub description: &'static str,
    pub category: Category,
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

        let is_compatible = get_latest_version_date(loader, &mod_id, version).await;
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
                err!(no_log, "{}", err.summary());
                false
            }
            Err(err) => {
                err!(no_log, "{err}");
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
                info!(no_log, "Cancelled recommended mod check");
                return None;
            }
        }

        is_compatible.then_some(self)
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum Category {
    Optimization,
    Utility,
    Visual,
}

impl Category {
    pub const ALL: &'static [Self] = &[Self::Optimization, Self::Utility, Self::Visual];
}

impl std::fmt::Display for Category {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Category::Optimization => "Optimization",
                Category::Utility => "Utility",
                Category::Visual => "Visual",
            }
        )
    }
}

// Credit to Void98 (https://github.com/void90user) for many of these
pub const RECOMMENDED_MODS: &[RecommendedMod] = &[
    RecommendedMod {
        id: "AANobbMI",
        name: "Sodium",
        description: "Optimizes the rendering engine",
        category: Category::Optimization,
        enabled_by_default: true,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "gvQqBUqZ",
        name: "Lithium",
        description: "Optimizes the integrated server",
        category: Category::Optimization,
        enabled_by_default: true,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "mOgUt4GM",
        name: "Mod Menu",
        description: "A mod menu for managing mods",
        category: Category::Utility,
        enabled_by_default: true,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "NNAgCjsB",
        name: "Entity Culling",
        description: "Optimizes entity rendering",
        category: Category::Optimization,
        enabled_by_default: true,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "5ZwdcRci",
        name: "ImmediatelyFast",
        description: "Optimizes immediate mode rendering",
        category: Category::Optimization,
        enabled_by_default: true,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "qQyHxfxd",
        name: "No Chat Reports",
        description: "Disables chat reporting",
        category: Category::Utility,
        enabled_by_default: true,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "aC3cM3Vq",
        name: "Mouse Tweaks",
        description: "Improves inventory controls",
        category: Category::Utility,
        enabled_by_default: true,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "hvFnDODi",
        name: "LazyDFU",
        description: "Speeds up Minecraft start time",
        category: Category::Optimization,
        enabled_by_default: true,
        backend: StoreBackendType::Modrinth,
    },

    // Optional Extras
    RecommendedMod {
        id: "YL57xq9U",
        name: "Iris Shaders",
        description: "Adds Shaders to Minecraft",
        category: Category::Visual,
        enabled_by_default: false,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "1IjD5062",
        name: "Continuity",
        description: "Adds connected textures",
        category: Category::Visual,
        enabled_by_default: false,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "kzwxhsjp",
        name: "Accurate Block Placement Reborn",
        description: "Makes placing blocks more accurate (note: some servers don't allow this)",
        category: Category::Utility,
        enabled_by_default: false,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "yBW8D80W",
        name: "LambDynamicLights",
        description: "Adds dynamic lights",
        category: Category::Visual,
        enabled_by_default: false,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "bXX9h73M",
        name: "MidnightControls",
        description: "Adds controller (and touch) support",
        category: Category::Utility,
        enabled_by_default: false,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "8shC1gFX",
        name: "BetterF3",
        description: "Cleans up the debug (F3) screen",
        category: Category::Utility,
        enabled_by_default: false,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "EsAfCjCV",
        name: "AppleSkin",
        description: "Shows hunger and saturation values",
        category: Category::Utility,
        enabled_by_default: false,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "1bokaNcj",
        name: "Xaero's Minimap",
        description: "Adds a minimap to the game",
        category: Category::Utility,
        enabled_by_default: false,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "NcUtCpym",
        name: "Xaero's World Map",
        description: "Adds a world map to the game",
        category: Category::Utility,
        enabled_by_default: false,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "p8RJPJIC",
        name: "Ixeris",
        description: "Reduce frame drops when turning camera",
        category: Category::Optimization,
        enabled_by_default: false,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "ONZm0H7Y",
        name: "Better Block Entities",
        description: "Drastically improves block entity rendering",
        category: Category::Optimization,
        enabled_by_default: false,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "pSISfJ4O",
        name: "quick pack",
        description: "Improve datapack/resourcepack loading times",
        category: Category::Optimization,
        enabled_by_default: false,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "zNuzb72d",
        name: "Substrate",
        description: "Optimization of the bottom and/or top layer(s) of the world",
        category: Category::Optimization,
        enabled_by_default: false,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "Ps1zyz6x",
        name: "ScalableLux",
        description: "Improves the performance of light updates",
        category: Category::Optimization,
        enabled_by_default: false,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "RSeLon5O",
        name: "Particle Core",
        description: "Optimizes particles and their rendering",
        category: Category::Optimization,
        enabled_by_default: false,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "OmQzuQFa",
        name: "LazyDFU Reloaded",
        description: "Speeds up Minecraft start time (for new versions of the game)",
        category: Category::Optimization,
        enabled_by_default: false,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "VSNURh3q",
        name: "Concurrent Chunk Management Engine (C2ME)",
        description: "Improves the chunk performance (EXPERIMENTAL)",
        category: Category::Optimization,
        enabled_by_default: false,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "blWBX5n1",
        name: "kennytvs-epic-force-close-loading-screen-mod-for-fabric",
        description: "Instantly closes the loading terrain screen, reduces the resource pack loading screen time",
        category: Category::Utility,
        enabled_by_default: false,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "TjSm1wrD",
        name: "ModernFix-mVUS",
        description: "Various optimizations for the game",
        category: Category::Optimization,
        enabled_by_default: false,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "g96Z4WVZ",
        name: "BadOptimizations",
        description: "Optimization mod that focuses on things other than rendering",
        category: Category::Optimization,
        enabled_by_default: false,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "uXXizFIs",
        name: "FerriteCore",
        description: "Memory usage optimizations",
        category: Category::Optimization,
        enabled_by_default: false,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "vSEH1ERy",
        name: "ThreadTweak",
        description: "Improve and tweak Minecraft thread scheduling",
        category: Category::Optimization,
        enabled_by_default: false,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "uLbm7CG6",
        name: "Language Reload",
        description: "Reduces load times and adds fallbacks for languages",
        category: Category::Optimization,
        enabled_by_default: false,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "9mtu0sUO",
        name: "Fast IP Ping",
        description: "Faster server pinging times",
        category: Category::Optimization,
        enabled_by_default: false,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "4WWQxlQP",
        name: "ServerCore",
        description: "A mod that aims to optimize the minecraft server (singleplayer too)",
        category: Category::Optimization,
        enabled_by_default: false,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "hasdd01q",
        name: "NoisiumForked",
        description: "Optimises worldgen performance",
        category: Category::Optimization,
        enabled_by_default: false,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "cJlZ132G",
        name: "Chat Plus",
        description: "Adds EXTENSIVE amount of very useful features to the chat",
        category: Category::Utility,
        enabled_by_default: false,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "PtjYWJkn",
        name: "Sodium Extra",
        description: "Adds some extra settings and utilities to Sodium",
        category: Category::Utility,
        enabled_by_default: false,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "Bh37bMuy",
        name: "Reese's Sodium Options",
        description: "Better options menu for sodium",
        category: Category::Utility,
        enabled_by_default: false,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "LQ3K71Q1",
        name: "Dynamic FPS",
        description: "Reduce resource usage while Minecraft is in the background, idle, or on battery",
        category: Category::Utility,
        enabled_by_default: false,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "w7ThoJFB",
        name: "Zoomify",
        description: "A zoom mod with infinite customizability",
        category: Category::Utility,
        enabled_by_default: false,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "fuuu3xnx",
        name: "Searchables",
        description: "Adds a search bar to many elements of the game like keybinds menu",
        category: Category::Utility,
        enabled_by_default: false,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "2M01OLQq",
        name: "Shulker Box Tooltip",
        description: "View the contents of shulker boxes from your inventory",
        category: Category::Utility,
        enabled_by_default: false,
        backend: StoreBackendType::Modrinth,
    },

];

// Recommended Mod template
/*
   RecommendedMod {
       id: "",
       name: "",
       description: "",
       category: "",
       enabled_by_default: false,
       backend: StoreBackendType::Modrinth,
   },
*/
