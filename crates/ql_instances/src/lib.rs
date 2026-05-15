//! # A module for creating, managing and running Minecraft client instances
//!
//! This is a crate of
//! [Quantum Launcher](https://mrmayman.github.io/quantumlauncher)
//! for dealing with many operations of Minecraft instances.
//!
//! **Not recommended to use in your own projects!**
//!
//! This module contains functions to:
//! - Create an instance
//! - Launch the instance
//! - Update the launcher
//! - Read logs
//! - List versions available for download
//! - Authenticate with Microsoft Accounts
//!
//! We use [BetterJSONS](https://github.com/MCPHackers/BetterJSONs)
//! and [LaunchWrapper](https://github.com/MCPHackers/LaunchWrapper) for:
//! - Better platform compatibility (ARM, 32-bit, etc.)
//! - Many bugfixes
//! - Skin, Sound, Auth fixes for old versions
//! - Skin support from `ely.by` and `littleskin`
//!
//! # A rant about natives
//!
//! (probably not relevant for you)
//!
//! ## What are natives?
//! Natives are libraries that are platform-specific.
//! They are used by Minecraft to interface with the operating system.
//!
//! ## Types of natives
//! - `natives: *` - These are part of the main library
//!   but have a separate jar file included that is extracted to
//!   the `natives` folder.
//! - `name: *-natives-*` - They are a separate library
//!   whose jar file is extracted to the `natives` folder.
//! - `classifiers: *` - Once again, part of main library
//!   but have separate jar files for each OS. Just formatted
//!   differently in the JSON.
//!
//! For whatever reason, natives are arranged in an
//! **extremely** messy way in the JSONs.
//!
//! The library downloader is also extremely fragile and messy.
//! It's designed to cope with real world conditions,
//! but may not be ideal. Feel free to report bugs if found.

#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]

mod download;
mod instance;
mod json_profiles;

pub use download::{DownloadError, create_instance, repeat_stage};
pub use instance::{launch::launch, list_versions::list_versions, notes};
pub use ql_core::jarmod;
pub use ql_java_handler::delete_java_installs;
