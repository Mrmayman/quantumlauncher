pub mod fabric;
pub mod forge;
pub mod optifine;

pub mod asset_index;
pub mod instance_config;
pub mod manifest;
pub mod version;

pub use fabric::FabricJSON;
pub use optifine::{JsonOptifine, OptifineArguments, OptifineLibrary};

pub use asset_index::AssetIndexMap;
pub use instance_config::{GlobalSettings, InstanceConfigJson};
pub use manifest::Manifest;
pub use version::{VersionDetails, V_1_5_2, V_FABRIC_UNSUPPORTED, V_PRECLASSIC_LAST};
