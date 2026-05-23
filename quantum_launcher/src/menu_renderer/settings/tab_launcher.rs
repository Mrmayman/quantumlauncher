use iced::{
    Alignment, Length,
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
                widget::horizontal_space(),
                button_with_icon(icons::folder_s(14), "Open Launcher Folder", 14)
                    .padding([5, 10])
                    .on_press_with(|| Message::CoreOpenPath(LAUNCHER_DIR.clone())),
            ]],
            opt_caching(config),
            column![
                row![
                    button_with_icon(icons::bin_s(12), "Clean unused assets", 12)
                        .padding([5, 10])
                        .on_press(LauncherSettingsMessage::CleanAssets.into()),
                ]
                .push_maybe(
                    self.cleaned_bytes
                        .as_ref()
                        .map(|size| widget::text!("Cleaned {size}!").size(14))
                )
                .align_y(Alignment::Center)
                .spacing(10),
                widget::row![
                    button_with_icon(icons::bin_s(12), "Clear Java installs", 12)
                        .padding([5, 10])
                        .on_press(LauncherSettingsMessage::ClearJavaInstalls.into()),
                    widget::text(
                        "Might fix some Java problems.\nPerfectly safe, will be redownloaded."
                    )
                    .style(tsubtitle)
                    .size(12),
                ]
                .spacing(10)
                .wrap(),
            ]
            .spacing(16),
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
            button_with_icon(icons::bin_s(12), "Clear Download Cache", 12)
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
