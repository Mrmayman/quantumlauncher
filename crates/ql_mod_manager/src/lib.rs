//! # A crate for dealing with Minecraft mods
//!
//! **Not recommended to use this in your own projects!**
//!
//! This crate provides a way to manage mods for
//! [Quantum Launcher](https://mrmayman.github.io/quantumlauncher).
//!
//! # Features
//!
//! - Interacting with Modrinth and Curseforge API to
//!   search, install, uninstall and update mods.
//! - Packaging mods into single-file presets
//!   (see [`Preset`] for more info)
//!
//! ## Installing and uninstalling:
//! - [Fabric](https://fabricmc.net/) (+ Legacy Fabric, etc)
//! - [Forge](https://files.minecraftforge.net/)
//! - [Optifine](https://optifine.net/)
//! - [Quilt](https://quiltmc.org/)
//! - [NeoForge](https://neoforged.net/)
//! - [Paper](https://papermc.io/) (for servers)

/// Installers and Uninstallers for loaders (Fabric/Forge/Optifine/Quilt/Paper).
pub mod loaders;
mod presets;
mod rate_limiter;
/// Mod manager integrated with Modrinth and Curseforge.
pub mod store;

pub use presets::{Preset, PresetOutput};
pub use store::add_files;
