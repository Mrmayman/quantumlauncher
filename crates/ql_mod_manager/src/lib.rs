//! # A crate for dealing with Minecraft mods
//!
//! **Not recommended to use this in your own projects!**
//!
//! This crate provides a way to manage mods for
//! [Quantum Launcher](https://mrmayman.github.io/quantumlauncher).
//!
//! # Features
//! - Interacting with Modrinth and Curseforge API to
//!   search, install, uninstall and update mods.
//! - Packaging mods into single-file presets
//!   (see [`PresetJson`] for more info)
//! ## Installing and uninstalling:
//! - Fabric
//! - Forge
//! - Optifine
//! - Quilt
//! - NeoForge
//! - Paper (for servers)

#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]

/// Installers and Uninstallers for loaders (Fabric/Forge/Optifine/Quilt/Paper).
pub mod loaders;
mod presets;
mod rate_limiter;
/// Mod manager integrated with Modrinth and Curseforge.
pub mod store;
pub use presets::{Preset, PresetOutput};
pub use store::add_files;
