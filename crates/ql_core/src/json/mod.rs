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
pub use version::VersionDetails;

pub const V_PRECLASSIC_LAST: &str = "2009-05-16T11:48:00+00:00";
pub const V_OFFICIAL_FABRIC_SUPPORT: &str = "2018-10-24T10:52:16+00:00";
/// Minecraft Alpha 1.0.15 release date
///
/// First version with multiplayer support in alpha
pub const V_A_1_0_15: &str = "2010-08-03T19:47:25+00:00";
pub const V_1_5_2: &str = "2013-04-25T15:45:00+00:00";
pub const V_1_12_2: &str = "2017-09-18T08:39:46+00:00";
pub const V_1_20_2: &str = "2023-09-20T09:02:57+00:00";
pub const V_PAULSCODE_LAST: &str = "2019-03-14T14:26:23+00:00";
/// Minecraft 13w23b release date (1.6.1 snapshot)
///
/// Last version with Texture Packs instead of Resource Packs
pub const V_LAST_TEXTUREPACK: &str = "2013-06-08T00:32:01+00:00";
