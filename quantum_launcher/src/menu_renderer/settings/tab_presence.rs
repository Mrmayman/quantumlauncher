use iced::{
    Length,
    widget::{self, column, row},
};

use crate::{
    config::{LauncherConfig, discord_rpc::RpcText},
    icons,
    menu_renderer::{Column, button_with_icon, checkered_list, tsubtitle},
    state::{MenuLauncherSettings, RpcInnerMessage, RpcMessage},
};

impl MenuLauncherSettings {
    pub(super) fn view_presence_tab<'a>(
        &'a self,
        config: &'a LauncherConfig,
        is_presence_running: bool,
    ) -> Column<'a> {
        let rpc_config = config.discord_rpc.clone().unwrap_or_default();

        checkered_list([
            column![
                row![
                    widget::text("Discord Rich Presence").size(20).width(Length::Fill),
                    button_with_icon(icons::refresh_s(14), "Reset to Defaults", 14)
                        .on_press_with(|| RpcMessage::ResetPresence.into()),
                ]
            ],

            column![
                widget::checkbox("Enable Broadcast", rpc_config.enable)
                    .on_toggle(|n| RpcMessage::Toggle(n).into()),
                widget::text("Sometimes toggling this option might take some time to apply the activity updates on Discord.").size(12).style(tsubtitle),
                widget::Space::with_height(3),
                widget::text("Please note that for all types of presences, the top text is mandatory. If not provided, the presence isn't set.").size(12).style(tsubtitle),
                widget::Space::with_height(5),
                    if is_presence_running {
                        row!(
                            icons::version_tick_s(13),
                            widget::Space::with_width(5),
                            widget::text("Synced!").size(13).style(tsubtitle)
                        )
                    } else if rpc_config.enable {
                        row!(
                            icons::clock_s(13),
                            widget::Space::with_width(5),
                            widget::text("Awaiting sync...").size(13).style(tsubtitle),
                        )
                    } else {
                        row!(
                            icons::cross_s(13),
                            widget::Space::with_width(5),
                            widget::text("Not enabled.").size(13).style(tsubtitle)
                        )
                    }

            ]
            .spacing(5),

            column![
                rpc_config.basic.view("Custom Presence", RpcMessage::DefaultChanged),
                if rpc_config.enable {
                    column![
                        button_with_icon(icons::discord_s(16), "Set Now", 12)
                            .padding([5, 10])
                            .on_press(RpcMessage::SetPresenceNow.into()),
                        widget::text("Changes will take effect on launcher restart or with the press of the button above.").size(12).style(tsubtitle),
                    ].spacing(5)
                } else {
                    column![]
                },
            ].spacing(10),

            column![
                widget::text("Toggles:"),
                widget::Space::with_height(5),
                widget::checkbox("Change presence during play/quit events", rpc_config.update_on_game_open)
                    .on_toggle(|n| RpcMessage::TogglePresenceOnGameEvent(n).into()),
                widget::text("Disabling this will ensure that only the custom rich presence set above stays alive when you run the launcher and/or play Minecraft.").size(12).style(tsubtitle),

            ].spacing(5),

            if rpc_config.update_on_game_open {
                widget::column![
                    widget::text("Event Presences:"),
                    widget::text("NOTE: You can use substitutes like ${instance} and ${version} for instance and version names respectively.").size(12).style(tsubtitle),
                    widget::Space::with_height(6),
                    rpc_config.on_gameopen.view("Game Launch", RpcMessage::GameOpen),
                    widget::Space::with_height(3),
                    rpc_config.on_gameexit.view("Game Exit", RpcMessage::GameExit),
                ].spacing(5)
            } else {
                widget::column![]
            }
        ])
    }
}

impl RpcText {
    fn view<'a>(
        &self,
        label: &str,
        m: impl Fn(RpcInnerMessage) -> RpcMessage + 'a + Clone,
    ) -> Column<'a> {
        let m2 = m.clone();
        column![
            widget::text(format!(
                "{label} {}",
                if self.top_text.is_none() {
                    "(Disabled)"
                } else {
                    ""
                }
            )),
            widget::Space::with_height(0),
            widget::text_input(
                "(Mandatory) Top Text",
                self.top_text.as_deref().unwrap_or_default()
            )
            .size(14)
            .on_input(move |v| m2(RpcInnerMessage::TopTextChanged(v)).into()),
            widget::text_input("Bottom Text", &self.bottom_text)
                .size(13)
                .on_input(move |v| m(RpcInnerMessage::BottomTextChanged(v)).into()),
        ]
        .spacing(5)
    }
}
