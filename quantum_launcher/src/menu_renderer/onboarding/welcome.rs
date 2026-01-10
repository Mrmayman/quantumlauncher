use iced::{widget, Alignment};
use ql_instances::auth::AccountType;

use crate::{
    config::LauncherConfig,
    icons,
    menu_renderer::{
        button_with_icon, center_x, get_mode_selector, onboarding::x86_warning,
        settings::get_theme_selector, Element, DISCORD,
    },
    state::{AccountMessage, MainMenuMessage, MenuWelcome, Message},
};

use super::IMG_LOGO;

impl MenuWelcome {
    pub fn view<'a>(&'a self, config: &'a LauncherConfig) -> Element<'a> {
        match self {
            MenuWelcome::P1InitialScreen => widget::column![
                widget::vertical_space(),
                center_x(widget::image(IMG_LOGO.clone()).width(200)),
                center_x(widget::text("Welcome to QuantumLauncher!").size(20)),
                center_x(widget::button("Get Started").on_press(Message::WelcomeContinueToTheme)),
            ]
            .push_maybe(cfg!(target_arch = "x86").then(|| center_x(x86_warning())))
            .push(widget::vertical_space())
            .align_x(iced::alignment::Horizontal::Center)
            .spacing(10)
            .into(),
            MenuWelcome::P2Theme => widget::column![
                widget::vertical_space(),
                center_x(widget::text("Customize your launcher!").size(24)),
                widget::row![
                    widget::horizontal_space(),
                    "Select Theme:",
                    get_mode_selector(config),
                    widget::horizontal_space(),
                ]
                .align_y(Alignment::Center)
                .spacing(10),
                widget::row![
                    widget::horizontal_space(),
                    "Select Color Scheme:",
                    widget::row![get_theme_selector().wrap()].width(250),
                    widget::horizontal_space(),
                ]
                .spacing(10),
                widget::Space::with_height(5),
                widget::row![
                    widget::horizontal_space(),
                    "Oh, and also consider",
                    button_with_icon(icons::discord(), "Join our Discord", 14)
                        .padding([4, 8])
                        .on_press(Message::CoreOpenLink(DISCORD.to_owned())),
                    widget::horizontal_space(),
                ]
                .align_y(Alignment::Center)
                .spacing(10),
                widget::Space::with_height(5),
                center_x(widget::button("Continue").on_press(Message::WelcomeContinueToAuth)),
                widget::vertical_space(),
            ]
            .spacing(10)
            .into(),
            MenuWelcome::P3Auth => {
                let next = Message::MScreenOpen {
                    message: Some("Install Minecraft by clicking \"+ New\"".to_owned()),
                    clear_selection: true,
                    is_server: Some(false),
                };
                widget::column![
                    widget::vertical_space(),
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
                    widget::Space::with_height(7),
                    center_x(widget::text("OR").size(20)),
                    widget::Space::with_height(7),
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
                    widget::vertical_space(),
                ]
                .spacing(5)
                .into()
            }
        }
    }
}
