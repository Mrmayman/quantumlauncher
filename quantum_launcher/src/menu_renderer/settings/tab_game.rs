use iced::{
    Length,
    widget::{self, column, row},
};
use ql_core::LAUNCHER_DIR;

use crate::{
    config::{AfterLaunchBehavior, LauncherConfig},
    icons,
    menu_renderer::{
        Column, button_with_icon, checkered_list,
        edit_instance::{args_split_by_space, get_args_list, resolution_dialog},
        settings::PREFIX_EXPLANATION,
        tsubtitle,
    },
    state::{LauncherSettingsMessage, MenuLauncherSettings, Message},
};

impl MenuLauncherSettings {
    pub(super) fn view_game_tab<'a>(&'a self, config: &'a LauncherConfig) -> Column<'a> {
        checkered_list([
            column![row![
                widget::text("Game").size(20).width(Length::Fill),
                button_with_icon(icons::folder_s(14), "Open Launcher Folder", 14)
                    .on_press_with(|| Message::CoreOpenPath(LAUNCHER_DIR.clone())),
            ]],
            opt_changelog(config),
            opt_after_launch(config),
            opt_resolution(config),
            opt_java_args(config),
            column![
                "Global Pre-Launch Prefix:",
                widget::text(PREFIX_EXPLANATION).size(12).style(tsubtitle),
                get_args_list(
                    config
                        .global_settings
                        .as_ref()
                        .and_then(|n| n.pre_launch_prefix.as_deref()),
                    |n| LauncherSettingsMessage::GlobalPreLaunchPrefix(n).into(),
                ),
                args_split_by_space(self.arg_split_by_space),
            ]
            .spacing(10),
            column![
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
                .wrap()
            ],
        ])
    }
}

fn opt_java_args(config: &LauncherConfig) -> Column<'_> {
    column![
        "Global Java Arguments:",
        get_args_list(config.extra_java_args.as_deref(), |msg| {
            LauncherSettingsMessage::GlobalJavaArgs(msg).into()
        }),
    ]
    .spacing(10)
}

fn opt_resolution(config: &LauncherConfig) -> Column<'_> {
    resolution_dialog(
        config.global_settings.as_ref(),
        |n| LauncherSettingsMessage::DefaultMinecraftWidthChanged(n).into(),
        |n| LauncherSettingsMessage::DefaultMinecraftHeightChanged(n).into(),
    )
}

fn opt_after_launch(config: &LauncherConfig) -> Column<'_> {
    let radio = |beh: AfterLaunchBehavior| {
        widget::radio(
            beh.desc(),
            beh,
            Some(config.c_after_launch_behavior()),
            |n| LauncherSettingsMessage::AfterLaunchBehaviorChanged(n).into(),
        )
        .size(14)
        .text_size(14)
    };

    column![
        row![
            widget::text("After game opens:").size(14),
            column![
                radio(AfterLaunchBehavior::DoNothing),
                radio(AfterLaunchBehavior::CloseLauncher),
                radio(AfterLaunchBehavior::MinimizeLauncher),
            ]
            .spacing(4),
        ]
        .spacing(10)
    ]
}

fn opt_changelog(config: &LauncherConfig) -> Column<'_> {
    column![
        widget::checkbox(
            "Write changelog after mod updates",
            config
                .persistent
                .clone()
                .unwrap_or_default()
                .write_mod_update_changelog,
        )
        .on_toggle(|n| LauncherSettingsMessage::ToggleModUpdateChangelog(n).into()),
        widget::text("Writes mod update changes to .minecraft/changelogs")
            .size(12)
            .style(tsubtitle),
    ]
    .spacing(5)
}
