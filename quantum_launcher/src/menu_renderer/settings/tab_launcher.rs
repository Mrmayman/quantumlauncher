use iced::{
    Length,
    widget::{self, column, row},
};
use ql_core::LAUNCHER_DIR;

use crate::{
    config::LauncherConfig,
    icons,
    menu_renderer::{Column, button_with_icon, checkered_list, tsubtitle},
    state::{LauncherSettingsMessage, MenuLauncherSettings, Message},
};

impl MenuLauncherSettings {
    pub(super) fn view_launcher_tab<'a>(&'a self, config: &'a LauncherConfig) -> Column<'a> {
        checkered_list([
            column![row![
                widget::text("Launcher Settings")
                    .size(20)
                    .width(Length::Fill),
                button_with_icon(icons::folder_s(14), "Open Launcher Folder", 14)
                    .on_press_with(|| Message::CoreOpenPath(LAUNCHER_DIR.clone())),
            ]],
            opt_caching(config),
        ])
    }
}

fn opt_caching(config: &LauncherConfig) -> Column<'_> {
    column![
        "Caching:",
        widget::Space::with_height(5),
        widget::checkbox(
            "Enable persistent, on-disk caching - Requires Restart",
            config.do_cache.clone().unwrap_or(true),
        )
        .on_toggle(|n| LauncherSettingsMessage::ToggleCaching(n).into()),
        widget::text(
            "Enables a fast cache for downloading mods, resource packs, game jars and more."
        )
        .size(12)
        .style(tsubtitle),
        widget::Space::with_height(5),
        widget::row![
            button_with_icon(icons::bin_s(12), "Clear download cache", 12)
                .padding([5, 10])
                .on_press(LauncherSettingsMessage::ClearDownloadCache.into()),
            widget::text(
                "Erases the cache for downloaded content (instances, mods, resource packs etc.)."
            )
            .style(tsubtitle)
            .size(12),
        ]
        .spacing(10)
        .wrap()
    ]
    .spacing(5)
}
