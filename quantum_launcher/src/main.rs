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
#![allow(clippy::doc_markdown)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_precision_loss)]

use std::{borrow::Cow, time::Duration};

use config::LauncherConfig;
use filthy_rich::DiscordIPC;
use iced::{Settings, Task};
use owo_colors::OwoColorize;
use state::{get_entries, Launcher, Message};

use ql_core::{constants::OS_NAME, err, file_utils, info, pt, IntoStringError, JsonFileError};

use crate::{
    menu_renderer::FONT_DEFAULT,
    state::{CustomJarState, State},
};

/// The CLI interface of the launcher.
mod cli;
/// Launcher configuration (global).
mod config;
/// Definitions of certain icons (like Download,
/// Play, Settings and so on) as `iced::widget`.
mod icons;
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

/// Handles `mclo.gs` log uploads
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
        #[cfg(feature = "auto_update")]
        let check_for_updates_command = Task::perform(
            async move { ql_instances::check_for_launcher_updates().await.strerr() },
            Message::UpdateCheckResult,
        );
        #[cfg(not(feature = "auto_update"))]
        let check_for_updates_command = Task::none();

        // discord rich presence identity
        let init_discord_ipc = if config
            .as_ref()
            .is_ok_and(|f| f.rich_presence.unwrap_or(true))
        {
            const DISCORD_APP_ID: &str = "1468876407756029965";

            Task::perform(
                async {
                    loop {
                        if let Ok(client) = DiscordIPC::new(DISCORD_APP_ID).await {
                            return Ok(client);
                        }

                        tokio::time::sleep(Duration::from_secs(3)).await
                    }
                },
                Message::DiscordIPCClientLaunched,
            )
        } else {
            Task::none()
        };

        let get_entries_command = Task::perform(get_entries(false), Message::CoreListLoaded);
        let mut launcher =
            Launcher::load_new(None, is_new_user, config).unwrap_or_else(Launcher::with_error);

        let load_notes_command = if let (Some(instance), State::Launch(menu)) =
            (launcher.selected_instance.clone(), &mut launcher.state)
        {
            menu.reload_notes(instance)
        } else {
            Task::none()
        };

        (
            launcher,
            Task::batch([
                check_for_updates_command,
                get_entries_command,
                load_notes_command,
                init_discord_ipc,
                Task::perform(ql_core::clean::dir("logs"), |n| {
                    Message::CoreCleanComplete(n.strerr())
                }),
                Task::perform(ql_core::clean::dir("downloads/cache"), |n| {
                    Message::CoreCleanComplete(n.strerr())
                }),
                CustomJarState::load(),
            ]),
        )
    }

    #[allow(clippy::unused_self)]
    fn subscription(&self) -> iced::Subscription<Message> {
        let tick = iced::time::every(Duration::from_millis(
            1000 / self.config.ui.unwrap_or_default().get_idle_fps(),
        ))
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

    info!(no_log, "Starting up the launcher... (OS: {OS_NAME})");
    if let Some(dir) = &launcher_dir {
        pt!(
            no_log,
            "{}",
            dir.to_string_lossy().bright_black().underline()
        );
    }

    let icon = load_icon();
    let config = load_config(launcher_dir.is_some());

    let c = config.as_ref().cloned().unwrap_or_default();
    let decorations = c.uses_system_decorations();
    let (width, height) = c.c_window_size();

    iced::application("QuantumLauncher", Launcher::update, Launcher::view)
        .subscription(Launcher::subscription)
        .scale_factor(Launcher::scale_factor)
        .theme(Launcher::theme)
        .settings(Settings {
            fonts: load_fonts(),
            default_font: FONT_DEFAULT,
            antialiasing: config
                .as_ref()
                .ok()
                .and_then(|n| n.ui_antialiasing)
                .unwrap_or(true),
            ..Default::default()
        })
        .window(iced::window::Settings {
            icon,
            exit_on_close_request: false,
            size: iced::Size { width, height },
            min_size: Some(iced::Size {
                width: 420.0,
                height: 310.0,
            }),
            decorations,
            transparent: true,
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
    match iced::window::icon::from_file_data(LAUNCHER_ICON, None) {
        Ok(n) => Some(n),
        Err(err) => {
            err!(no_log, "Couldn't load launcher icon! (bug detected): {err}");
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

/// This is the only `unsafe` Rust code in the entire launcher.
/// It tweaks Windows terminal behaviour so that:
///
/// - If launcher is opened from terminal,
///   it shows output in terminal
/// - If it's opened normally from GUI,
///   no terminal window pops up
///
/// Basically Linux-default behaviour.
#[cfg(windows)]
fn attach_to_console() {
    use windows::Win32::System::Console::AttachConsole;
    use windows::Win32::System::Console::ATTACH_PARENT_PROCESS;

    unsafe {
        // No one cares if it fails. Ignore the `Result<()>`
        _ = AttachConsole(ATTACH_PARENT_PROCESS);
    }
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
fn should_migrate() -> bool {
    let Some(legacy_dir) = file_utils::migration_legacy_launcher_dir() else {
        return false;
    };

    // Already migrated or haven't run the launcher before migration
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
            eprintln!("Migration failed: {e}");
        } else if let Err(e) = file_utils::create_symlink(&new_dir, &legacy_dir) {
            eprintln!("Migration successful but couldn't create symlink to the legacy dir: {e}");
        } else {
            ql_core::info!("Migration successful!\nYour launcher files are now in ~./local/share/QuantumLauncher");
        }
    }
}
