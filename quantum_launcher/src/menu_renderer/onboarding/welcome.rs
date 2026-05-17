use iced::{
    Alignment, Length,
    widget::{self, column, row},
};
use ql_instances::auth::AccountType;

use crate::{
    config::LauncherConfig,
    icons,
    menu_renderer::{
        DISCORD, Element, button_with_icon, center_x, get_mode_selector, onboarding::x86_warning,
        settings::get_theme_selector, tsubtitle,
    },
    state::{AccountMessage, MainMenuMessage, MenuWelcome, Message, RpcMessage},
};

use super::IMG_LOGO;

impl MenuWelcome {
    pub fn view<'a>(&'a self, config: &'a LauncherConfig) -> Element<'a> {
        match self {
            MenuWelcome::P1InitialScreen => column![
                widget::space().height(Length::Fill),
                row![
                    widget::space().width(Length::Fill),
                    widget::image(IMG_LOGO.clone()).width(100),
                    column![
                        widget::text("Welcome to"),
                        widget::text("QuantumLauncher").size(32),
                    ],
                    widget::space().width(Length::Fill),
                ]
                .align_y(Alignment::Center),
                if cfg!(target_arch = "x86") {
                    let e: Element = x86_warning().into();
                    e
                } else {
                    widget::text("Play Minecraft your own way!")
                        .size(14)
                        .style(tsubtitle)
                        .into()
                },
                widget::space().height(2),
                button_with_icon(icons::play(), "Get Started", 16)
                    .on_press(Message::WelcomeContinueToTheme),
                widget::space().height(Length::Fill)
            ]
            .width(Length::Fill)
            .align_x(Alignment::Center)
            .spacing(10)
            .into(),
            MenuWelcome::P2Theme => column![
                widget::space().height(Length::Fill),
                center_x(widget::text("Customize your launcher!").size(24)),
                widget::row!["Mode:", get_mode_selector(config)]
                    .align_y(Alignment::Center)
                    .spacing(10),
                widget::row![
                    widget::space().width(20),
                    "Theme:",
                    get_theme_selector().wrap()
                ]
                .width(350)
                .spacing(10),
                widget::space().height(5),
                column![
                    widget::toggler(config.c_rpc_enabled())
                        .label("Enable Discord Rich Presence")
                        .on_toggle(|t| RpcMessage::Toggle(t).into()),
                    widget::text("Allow others to see your gameplay activity on Discord")
                        .size(12)
                        .style(tsubtitle),
                ]
                .spacing(5),
                widget::space().height(5),
                button_with_icon(icons::discord(), "Join our Discord", 14)
                    .padding([4, 8])
                    .on_press(Message::CoreOpenLink(DISCORD.to_owned())),
                widget::space().height(5),
                center_x(widget::button("Continue").on_press(Message::WelcomeContinueToAuth)),
                widget::space().height(Length::Fill),
            ]
            .width(Length::Fill)
            .align_x(Alignment::Center)
            .spacing(10)
            .into(),
            MenuWelcome::P3Auth => {
                let next = Message::MScreenOpen {
                    message: None,
                    clear_selection: true,
                };
                column![
                    widget::space().height(Length::Fill),
                    center_x(
                        widget::button("Login to Microsoft").on_press(Message::Account(
                            AccountMessage::OpenMenu {
                                is_from_welcome_screen: true,
                                kind: AccountType::Microsoft
                            }
                        ))
                    ),
                    center_x(widget::button("Login to ely.by").on_press(Message::Account(
                        AccountMessage::OpenMenu {
                            is_from_welcome_screen: true,
                            kind: AccountType::ElyBy
                        }
                    ))),
                    center_x(
                        widget::button("Login to littleskin").on_press(Message::Account(
                            AccountMessage::OpenMenu {
                                is_from_welcome_screen: true,
                                kind: AccountType::LittleSkin
                            }
                        ))
                    ),
                    widget::space().height(7),
                    center_x(widget::text("OR").size(20)),
                    widget::space().height(7),
                    center_x(
                        widget::text_input("Enter username...", &config.username)
                            .width(200)
                            .on_input(|t| MainMenuMessage::UsernameSet(t).into())
                            .on_submit_maybe((!config.username.is_empty()).then(|| next.clone()))
                    ),
                    center_x(
                        widget::button(center_x("Continue"))
                            .width(200)
                            .on_press_maybe((!config.username.is_empty()).then(|| next.clone()))
                    ),
                    widget::space().height(Length::Fill),
                ]
                .spacing(5)
                .into()
            }
        }
    }
}
