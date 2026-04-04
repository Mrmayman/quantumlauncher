use std::sync::LazyLock;

use iced::{Length, widget};

use super::{Element, back_button, back_to_launch_screen, sidebar, sidebar_button};
use crate::{
    config::LauncherConfig,
    icons,
    state::{LauncherSettingsMessage, LauncherSettingsTab, MenuLauncherSettings, Message},
    stylesheet::{
        styles::{LauncherTheme, LauncherThemeColor},
        widgets::StyleButton,
    },
};

mod tab_about;
mod tab_game;
mod tab_ui;

pub static IMG_ICED: LazyLock<widget::image::Handle> = LazyLock::new(|| {
    widget::image::Handle::from_bytes(include_bytes!("../../../../assets/iced.png").as_slice())
});

pub const PREFIX_EXPLANATION: &str =
    "Commands to add before the game launch command\nEg: prime-run/gamemoderun/mangohud";

impl MenuLauncherSettings {
    pub fn view<'a>(&'a self, config: &'a LauncherConfig) -> Element<'a> {
        widget::row![
            sidebar(
                "MenuLauncherSettings:sidebar",
                Some(
                    widget::column![
                        back_button().on_press(back_to_launch_screen(None, None)),
                        Self::get_heading()
                    ]
                    .spacing(10)
                    .into()
                ),
                LauncherSettingsTab::ALL.iter().map(|tab| {
                    let text = widget::text(tab.to_string());
                    sidebar_button(
                        tab,
                        &self.selected_tab,
                        text,
                        LauncherSettingsMessage::ChangeTab(*tab).into(),
                    )
                })
            )
            .style(|_: &LauncherTheme| widget::container::Style {
                text_color: None,
                background: None,
                border: iced::Border::default(),
                shadow: iced::Shadow::default()
            }),
            widget::scrollable(self.selected_tab.view(config, self))
                .width(Length::Fill)
                .spacing(0)
                .style(LauncherTheme::style_scrollable_flat_dark)
        ]
        .into()
    }

    fn get_heading() -> widget::Row<'static, Message, LauncherTheme> {
        widget::row![icons::gear_s(20), widget::text("Settings").size(20)]
            .padding(iced::Padding {
                top: 5.0,
                right: 0.0,
                bottom: 2.0,
                left: 10.0,
            })
            .spacing(10)
    }
}

pub fn get_theme_selector() -> widget::Row<'static, Message, LauncherTheme> {
    widget::row(LauncherThemeColor::ALL.iter().map(|color| {
        widget::button(widget::text(color.to_string()).size(13))
            .padding([2, 4])
            .style(|theme: &LauncherTheme, s| {
                LauncherTheme {
                    color: *color,
                    alpha: 1.0,
                    ..*theme
                }
                .style_button(s, StyleButton::Round)
            })
            .on_press(Message::LauncherSettings(
                LauncherSettingsMessage::ColorSchemePicked(*color),
            ))
            .into()
    }))
    .spacing(5)
}

impl LauncherSettingsTab {
    pub fn view<'a>(
        &'a self,
        config: &'a LauncherConfig,
        menu: &'a MenuLauncherSettings,
    ) -> Element<'a> {
        match self {
            LauncherSettingsTab::UserInterface => menu.view_ui_tab(config),
            LauncherSettingsTab::Game => menu.view_game_tab(config),
            LauncherSettingsTab::About => tab_about::view(),
        }
        .into()
    }
}
