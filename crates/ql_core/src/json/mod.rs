pub mod fabric;
pub mod forge;
pub mod lwjgl;
pub mod optifine;

pub mod asset_index;
pub mod instance_config;
pub mod manifest;
pub mod version;

pub use fabric::FabricJSON;
pub use lwjgl::{LwjglVersion, LwjglVersionList};
pub use optifine::{JsonOptifine, OptifineArguments, OptifineLibrary};

pub use asset_index::AssetIndex;
pub use instance_config::{GlobalSettings, InstanceConfigJson};
pub use manifest::Manifest;
pub use version::{
    VersionDetails, V_1_12_2, V_1_5_2, V_OFFICIAL_FABRIC_SUPPORT, V_PAULSCODE_LAST,
    V_PRECLASSIC_LAST,
};
