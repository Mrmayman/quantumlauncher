use iced::widget::{self, column, text};

use crate::{
    config::LauncherConfig,
    menu_renderer::{Element, tsubtitle},
    state::RpcMessage,
};

pub fn changelog(config: &LauncherConfig) -> Element<'static> {
    column![
        text("Welcome to QuantumLauncher TBD!").size(40),

        column![
            widget::toggler(config.c_rpc_enabled())
                .label("Enable Discord Rich Presence")
                .on_toggle(
                    |t| RpcMessage::Toggle(t).into()
                ),
            widget::text("Allow others to see your gameplay activity in Discord\nMore info below...")
                .size(12)
                .style(tsubtitle),
        ]
        .spacing(5),

        widget::container(column![
            "TLDR;",
            text("- Instance folders for better organization").size(14),
            text("- One-click shortcuts: launch without opening the launcher!").size(14),
            text("- Numerous UX improvements and bug fixes").size(14),
        ].spacing(5)).padding(10),

        text("Mod Store").size(32),
        column![].spacing(5),

        text("Discord Rich Presence").size(32),
        column![].spacing(10),

        widget::horizontal_rule(1),
        text("UX").size(32),
        column![].spacing(5),

        widget::horizontal_rule(1),
        text("Fixes").size(20),
        column![
            text("- Fixed \"system theme\" error spam on Raspberry Pi OS, LXDE, Openbox, etc").size(12),
            text("- Fixed launcher auto-updater not supporting `.tar.gz` files (only `.zip`)").size(12),
            text("- Fixed Modrinth pages sometimes appearing after selecting Curseforge, and vice versa").size(12),
            text("- Fixed mods installed through Curseforge modpacks internally being stored as Modrinth mods").size(12),
            text("- Fixed Java binary not being found on Linux ARM").size(12),
        ].spacing(5),

        widget::Space::with_height(10),
        text("Ready to experience your new launcher now? Hit continue!").size(20),
    ]
    .padding(10)
    .spacing(10)
    .into()
}
