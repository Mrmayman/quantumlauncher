use std::sync::Mutex;

use iced::{
    Length,
    widget::{self, column, row},
};

use crate::{
    config::{
        LauncherConfig,
        discord_rpc::{PresenceStatusDisplayType, RpcConfig, RpcText},
    },
    icons,
    menu_renderer::{Column, FONT_MONO, button_with_icon, checkered_list, tsubtitle},
    message_update::PresenceConnectionState,
    state::{MenuLauncherSettings, Message, RpcInnerMessage, RpcMessage},
    stylesheet::{styles::LauncherTheme, widgets::StyleButton},
};

impl MenuLauncherSettings {
    pub(super) fn view_presence_tab<'a>(
        &'a self,
        config: &'a LauncherConfig,
        discord_connection_state: &Mutex<PresenceConnectionState>,
    ) -> Column<'a> {
        let rpc_config = config.discord_rpc.clone().unwrap_or_default();
        let presence_state = discord_connection_state.lock().unwrap();

        let status = |icon, text| {
            row![
                icon,
                widget::Space::with_width(5),
                widget::text(text).size(13).style(tsubtitle)
            ]
        };

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
                widget::Space::with_height(5),
                match *presence_state {
                    PresenceConnectionState::Uninitialized => {
                        if rpc_config.enable {
                            status(icons::clock_s(13), "Waiting for Discord...")
                        } else {
                            status(icons::cross_s(13), "Not enabled.")
                        }
                    },
                    PresenceConnectionState::Connected => {
                        status(icons::clock_s(13), "Waiting for activity...")
                    },
                    PresenceConnectionState::Active => {
                        status(icons::version_tick_s(13), "Synced!")
                    },
                    PresenceConnectionState::Disconnected => {
                        status(icons::clock_s(13), "Disconnected, waiting for Discord...")
                    },
                },
            ]
            .spacing(5),

            column![
                widget::text("Core Settings:"),
                widget::text("Tweak initial/custom presence, add flavor, change names, let your imagination fly.").size(12).style(tsubtitle),
                widget::Space::with_height(6),

                column![
                    rpc_config.basic.view(&format!("{} Presence", if rpc_config.update_on_game_open {"Startup"} else {"Custom"}), RpcMessage::DefaultChanged),
                    if rpc_config.enable {
                        column![
                            button_with_icon(icons::discord_s(16), "Set Now", 12)
                                .padding([5, 10])
                                .on_press(RpcMessage::SetPresenceNow.into()),
                            widget::text("Changes will take effect on launcher restart or with the press of the button above.").size(12).style(tsubtitle),
                        ].spacing(5)
                    } else {
                        column![
                            widget::text("Toggle 'Enable Broadcast' to actually start using presences.").size(12).style(tsubtitle),
                        ]
                    },
                ].spacing(20),
            ].spacing(5),


            column![
                widget::text("Toggles:"),
                widget::Space::with_height(5),
                widget::checkbox("Change presence during play/quit events", rpc_config.update_on_game_open)
                    .on_toggle(|n| RpcMessage::TogglePresenceOnGameEvent(n).into()),
                widget::text("Disabling this will ensure that only the custom rich presence set above stays alive when you run the launcher and/or play Minecraft.").size(12).style(tsubtitle),
                widget::Space::with_height(5),
                row![
                    widget::checkbox("Competing Mode", rpc_config.competing)
                        .on_toggle(|n| RpcMessage::ToggleCompeting(n).into()),
                    widget::Space::with_width(5),
                    icons::paintbrush_s(15),
                ],
                widget::text("A fancier way to show off your activities. Try this at home!").size(12).style(tsubtitle),
            ].spacing(5),

            if rpc_config.update_on_game_open {
                widget::column![
                    widget::text("Event Presences:"),
                    widget::text("NOTE: You can use substitutes like ${instance} and ${version} for instance and version names respectively.").size(12).style(tsubtitle),
                    widget::Space::with_height(6),

                    widget::row![
                        rpc_config.on_gameopen.view("Game Launch", RpcMessage::GameOpen),
                        widget::Space::with_height(3),
                        rpc_config.on_gameexit.view("Game Exit", RpcMessage::GameExit),
                    ].spacing(10)
                ].spacing(5)
            } else {
                widget::column![]
            },

            column![
                widget::text("App/Activity Name"),
                widget::text("Replace the default 'QuantumLauncher' name with something else: ").size(12).style(tsubtitle),
                widget::Space::with_height(2),
                widget::text_input("(e.g. epic game)", rpc_config.name.as_deref().unwrap_or_default())
                    .size(21)
                    .on_input(|v| RpcMessage::SetName(v).into()),
                widget::Space::with_height(10),
                widget::text("Status Display Type"),
                widget::text("Choose which one to display in your profile banner as status:").size(12).style(tsubtitle),
                widget::Space::with_height(5),
                get_sdt_selector(&rpc_config)
            ].spacing(5),
        ])
    }
}

impl RpcText {
    fn view<'a>(
        &self,
        label: &str,
        m: impl Fn(RpcInnerMessage) -> RpcMessage + 'a + Clone,
    ) -> Column<'a> {
        let m1 = m.clone();
        let m2 = m.clone();
        let m3 = m.clone();

        let space = |e| row![widget::Space::with_width(10), e];

        let top_text = self.top_text.as_deref().unwrap_or_default();
        let bottom_text = self.bottom_text.as_deref().unwrap_or_default();

        column![
            widget::text(format!(
                "{label} {}",
                if self.top_text.is_none() && self.bottom_text.is_none() {
                    "(Disabled)"
                } else {
                    ""
                }
            )),
            widget::text_input("Top Text", top_text)
                .size(14)
                .on_input(move |v| m1(RpcInnerMessage::TopText(v)).into()),
        ]
        .push_maybe((!top_text.is_empty()).then(|| {
            space(
                widget::text_input(
                    "Top Text URL (optional)",
                    self.top_text_url.as_deref().unwrap_or_default(),
                )
                .size(14)
                .font(FONT_MONO)
                .on_input(move |v| m2(RpcInnerMessage::TopTextURL(v)).into()),
            )
        }))
        .push(
            widget::text_input("Bottom Text", bottom_text)
                .size(14)
                .on_input(move |v| m(RpcInnerMessage::BottomText(v)).into()),
        )
        .push_maybe((!bottom_text.is_empty()).then(|| {
            space(
                widget::text_input(
                    "Bottom Text URL (optional)",
                    self.bottom_text_url.as_deref().unwrap_or_default(),
                )
                .size(14)
                .font(FONT_MONO)
                .on_input(move |v| m3(RpcInnerMessage::BottomTextURL(v)).into()),
            )
        }))
        .spacing(5)
    }
}

pub fn get_sdt_selector(config: &RpcConfig) -> widget::Row<'static, Message, LauncherTheme> {
    const PADDING: iced::Padding = iced::Padding {
        top: 5.0,
        bottom: 5.0,
        right: 10.0,
        left: 10.0,
    };

    let s_dt = config.status_display_type;

    widget::row(PresenceStatusDisplayType::ALL.iter().map(|dt| {
        if *dt != s_dt {
            widget::button(widget::text(dt.to_string()).size(14))
                .padding(PADDING)
                .style(|theme: &LauncherTheme, s| {
                    LauncherTheme {
                        alpha: 1.0,
                        ..*theme
                    }
                    .style_button(s, StyleButton::Round)
                })
                .on_press(RpcMessage::StatusDisplayTypePicked(*dt).into())
                .into()
        } else {
            widget::container(row![
                icons::checkmark_s(15),
                widget::Space::with_width(5),
                widget::text(dt.to_string()).size(14)
            ])
            .padding(PADDING)
            .into()
        }
    }))
    .spacing(5)
}
