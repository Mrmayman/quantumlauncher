/*
QuantumLauncher
Copyright (C) 2024  Mrmayman & Contributors

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <http://www.gnu.org/licenses/>.
*/

#![doc = include_str!("../../README.md")]
#![windows_subsystem = "windows"]
#![allow(clippy::doc_nested_refdefs)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_precision_loss)]

use std::{borrow::Cow, time::Duration};

use config::LauncherConfig;
use iced::{futures::executor::block_on, Settings, Task};
use state::{get_entries, Launcher, Message, ServerProcess};

use ql_core::{err, err_no_log, file_utils, info, info_no_log, IntoStringError, JsonFileError};
use ql_instances::OS_NAME;
use tokio::io::AsyncWriteExt;

/// The CLI interface of the launcher.
mod cli;
/// Launcher configuration (global).
mod config;
/// Definitions of certain icons (like Download,
/// Play, Settings and so on) as `iced::widget`.
mod icon_manager;
/// All the main structs and enums used in the launcher.
mod state;

/// Code to handle all [`Message`]'s coming from
/// user interaction.
///
/// This and the [`view`] module together form
/// the Model-View-Controller pattern.
mod update;
/// Code to manage the rendering of menus overall
/// (this invokes [`menu_renderer`]).
///
/// This and the [`update`] module together form
/// the Model-View-Controller pattern.
mod view;

/// Code to render the specific menus
/// (called by [`view`]).
mod menu_renderer;

/// Handles mclo.gs log uploads
mod mclog_upload;
/// Child functions of the
/// [`Launcher::update`] function.
mod message_handler;
/// Handlers for "child messages".
///
/// The [`Message`] enum is split into
/// categories (like `Message::Account(AccountMessage)`).
///
/// This module has functions for handling each of
/// these "child messages".
mod message_update;
/// Handles mod store
mod mods_store;
/// Stylesheet definitions (launcher themes)
mod stylesheet;
/// Code to tick every frame
mod tick;

const LAUNCHER_ICON: &[u8] = include_bytes!("../../assets/icon/ql_logo.ico");

impl Launcher {
    fn new(
        is_new_user: bool,
        config: Result<LauncherConfig, JsonFileError>,
    ) -> (Self, Task<Message>) {
        let check_for_updates_command = Task::perform(
            async move { ql_instances::check_for_launcher_updates().await.strerr() },
            Message::UpdateCheckResult,
        );

        let get_entries_command = Task::perform(
            get_entries("instances".to_owned(), false),
            Message::CoreListLoaded,
        );

        let log_cmd = Task::perform(file_utils::clean_log_spam(), |n| {
            Message::CoreLogCleanComplete(n.strerr())
        });

        (
            Launcher::load_new(None, is_new_user, config).unwrap_or_else(Launcher::with_error),
            Task::batch([check_for_updates_command, get_entries_command, log_cmd]),
        )
    }

    fn kill_selected_server(&mut self, server: &str) {
        if let Some(ServerProcess {
            stdin: Some(stdin),
            is_classic_server,
            child,
            has_issued_stop_command,
            ..
        }) = self.server_processes.get_mut(server)
        {
            *has_issued_stop_command = true;
            if *is_classic_server {
                if let Err(err) = child.lock().unwrap().start_kill() {
                    err!("Could not kill classic server: {err}");
                }
            } else {
                let future = stdin.write_all("stop\n".as_bytes());
                _ = block_on(future);
            };
        }
    }

    // Iced expects a `fn(&self)` so we're putting `&self`
    // even when not needed.
    #[allow(clippy::unused_self)]
    fn subscription(&self) -> iced::Subscription<Message> {
        const UPDATES_PER_SECOND: u64 = 5;

        let tick = iced::time::every(Duration::from_millis(1000 / UPDATES_PER_SECOND))
            .map(|_| Message::CoreTick);

        let events = iced::event::listen_with(|a, b, _| Some(Message::CoreEvent(a, b)));

        iced::Subscription::batch(vec![tick, events])
    }

    fn theme(&self) -> stylesheet::styles::LauncherTheme {
        self.theme.clone()
    }

    fn scale_factor(&self) -> f64 {
        self.config.ui_scale.unwrap_or(1.0).max(0.05)
    }
}

const DEBUG_LOG_BUTTON_HEIGHT: f32 = 16.0;

const WINDOW_HEIGHT: f32 = 400.0;
const WINDOW_WIDTH: f32 = 600.0;

fn main() {
    #[cfg(target_os = "windows")]
    attach_to_console();
    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    if should_migrate() {
        do_migration();
    }

    let is_new_user = file_utils::is_new_user();
    // let is_new_user = true; // Uncomment to test the intro screen.

    let (launcher_dir, is_dir_err) = load_launcher_dir();
    cli::start_cli(is_dir_err);

    info_no_log!("Starting up the launcher... (OS: {OS_NAME})");

    let icon = load_icon();
    let mut config = load_config(launcher_dir.is_some());
    let scale = match &config {
        Ok(cfg) => cfg.ui_scale.unwrap_or(1.0),
        Err(_e) => 1.0,
    } as f32;
    let (width, height) = config.as_mut().ok().map_or(
        (WINDOW_WIDTH * scale, WINDOW_HEIGHT * scale),
        LauncherConfig::read_window_size,
    );

    iced::application("QuantumLauncher", Launcher::update, Launcher::view)
        .subscription(Launcher::subscription)
        .scale_factor(Launcher::scale_factor)
        .theme(Launcher::theme)
        .settings(Settings {
            fonts: load_fonts(),
            default_font: iced::Font::with_name("Inter"),
            antialiasing: config
                .as_ref()
                .ok()
                .and_then(|n| n.antialiasing)
                .unwrap_or(true),
            ..Default::default()
        })
        .window(iced::window::Settings {
            icon,
            exit_on_close_request: false,
            size: iced::Size { width, height },
            min_size: Some(iced::Size {
                width: 420.0,
                height: 300.0,
            }),
            ..Default::default()
        })
        .run_with(move || Launcher::new(is_new_user, config))
        .unwrap();
}

fn load_launcher_dir() -> (Option<std::path::PathBuf>, bool) {
    let launcher_dir_res = file_utils::get_launcher_dir();
    let mut launcher_dir = None;
    let is_dir_err = match launcher_dir_res {
        Ok(n) => {
            eprintln!("- Launcher dir: {}", n.display());
            launcher_dir = Some(n);
            false
        }
        Err(err) => {
            err!("Couldn't get launcher dir: {err}");
            true
        }
    };
    (launcher_dir, is_dir_err)
}

fn load_config(dir_is_ok: bool) -> Result<LauncherConfig, JsonFileError> {
    if let Some(cfg) = dir_is_ok.then(LauncherConfig::load_s) {
        cfg
    } else {
        Err(JsonFileError::Io(ql_core::IoError::LauncherDirNotFound))
    }
}

fn load_icon() -> Option<iced::window::Icon> {
    match iced::window::icon::from_file_data(LAUNCHER_ICON, Some(image::ImageFormat::Ico)) {
        Ok(n) => Some(n),
        Err(err) => {
            err_no_log!("Couldn't load launcher icon! (bug detected): {err}");
            None
        }
    }
}

fn load_fonts() -> Vec<Cow<'static, [u8]>> {
    vec![
        include_bytes!("../../assets/fonts/Inter-Regular.ttf")
            .as_slice()
            .into(),
        include_bytes!("../../assets/fonts/JetBrainsMono-Regular.ttf")
            .as_slice()
            .into(),
        include_bytes!("../../assets/fonts/password_asterisks/password-asterisks.ttf")
            .as_slice()
            .into(),
        include_bytes!("../../assets/fonts/icons.ttf")
            .as_slice()
            .into(),
    ]
}

#[cfg(windows)]
fn attach_to_console() {
    use windows::Win32::System::Console::AttachConsole;
    use windows::Win32::System::Console::ATTACH_PARENT_PROCESS;

    unsafe {
        _ = AttachConsole(ATTACH_PARENT_PROCESS);
    }
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
fn should_migrate() -> bool {
    let Some(legacy_dir) = file_utils::migration_legacy_launcher_dir() else {
        return false;
    };

    // Already migrated or haven't ran the launcher before migration
    // Don't load the config for no reason
    if legacy_dir.is_symlink() || !legacy_dir.exists() {
        return false;
    }

    let Some(new_dir) = file_utils::migration_launcher_dir() else {
        eprintln!("Failed to get new directory");
        return false;
    };

    if new_dir.join("config.json").exists() {
        eprintln!("Skipping migration: target config exists");
        false
    } else if legacy_dir == new_dir {
        eprintln!("Skipping migration: same directory");
        false
    } else {
        true
    }
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
fn do_migration() {
    // Can't use `info!` for logs,
    // since that runs get_logs_dir which lazy allocates LAUNCHER_DIR
    // which creates the new_dir and that would fail the migration
    println!("Running migration");
    if let (Some(legacy_dir), Some(new_dir)) = (
        file_utils::migration_legacy_launcher_dir(),
        file_utils::migration_launcher_dir(),
    ) {
        if let Err(e) = std::fs::rename(&legacy_dir, &new_dir) {
            eprintln!("Migration failed: {}", e);
        } else if let Err(e) = ql_core::file_utils::create_symlink(&new_dir, &legacy_dir) {
            eprintln!(
                "Migration successful but couldnt create symlink to the legacy dir: {e}",
            );
        } else {
            info!("Migration successful!\nYour launcher files are now in ~./local/share/QuantumLauncher")
        }
    }
}
