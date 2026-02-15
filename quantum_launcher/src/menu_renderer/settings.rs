use std::sync::LazyLock;

use iced::{widget, Alignment, Length};
use ql_core::{LAUNCHER_DIR, WEBSITE};

use super::{
    back_button, button_with_icon, get_mode_selector, sidebar_button, underline, Element, DISCORD,
    GITHUB,
};
use crate::menu_renderer::edit_instance::{args_split_by_space, get_args_list, resolution_dialog};
use crate::menu_renderer::{back_to_launch_screen, checkered_list, sidebar, tsubtitle};
use crate::{
    config::LauncherConfig,
    icons,
    state::{LauncherSettingsMessage, LauncherSettingsTab, MenuLauncherSettings, Message},
    stylesheet::{
        color::Color,
        styles::{LauncherTheme, LauncherThemeColor},
        widgets::StyleButton,
    },
};

pub static IMG_ICED: LazyLock<widget::image::Handle> = LazyLock::new(|| {
    widget::image::Handle::from_bytes(include_bytes!("../../../assets/iced.png").as_slice())
});

const SETTINGS_SPACING: f32 = 10.0;
const SETTING_WIDTH: u16 = 180;

pub const PREFIX_EXPLANATION: &str = "Commands to add before the game launch command\nEg: 'prime-run' to force NVIDIA GPU on Linux with Optimus";

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
                        Message::LauncherSettings(LauncherSettingsMessage::ChangeTab(*tab)),
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

    fn view_ui_tab<'a>(&'a self, config: &'a LauncherConfig) -> Element<'a> {
        let ui_scale_apply = widget::row![
            widget::horizontal_space(),
            widget::button(widget::text("Apply").size(12))
                .padding([1.8, 5.0])
                .on_press(Message::LauncherSettings(
                    LauncherSettingsMessage::UiScaleApply,
                ))
        ];

        let idle_fps = config.ui.unwrap_or_default().get_idle_fps();

        checkered_list::<Element>([
            widget::column![widget::text("User Interface").size(20)].into(),

            widget::column![
                widget::row!["Mode: ", get_mode_selector(config)]
                    .spacing(5)
                    .align_y(Alignment::Center),
                widget::Space::with_height(5),
                "Theme:",
                get_theme_selector().wrap()
            ]
            .spacing(5)
            .into(),
            widget::row![
                widget::row![widget::text!("UI Scale ({:.2}x)  ", self.temp_scale).size(15)]
                    .push_maybe(
                        ((self.temp_scale - config.ui_scale.unwrap_or(1.0)).abs() > 0.01)
                            .then_some(ui_scale_apply)
                    )
                    .align_y(Alignment::Center).width(SETTING_WIDTH),
                widget::slider(0.5..=2.0, self.temp_scale, |n| Message::LauncherSettings(
                    LauncherSettingsMessage::UiScale(n)
                ))
                .step(0.1),
            ]
            .align_y(Alignment::Center)
            .spacing(5)
            .into(),

            get_ui_opacity(config).into(),

            widget::column![
                // TODO: This requires launcher restart
                // widget::checkbox("Custom Window Decorations", !config.c_window_decorations()).on_toggle(|n| {
                //     Message::LauncherSettings(LauncherSettingsMessage::ToggleWindowDecorations(n))
                // }),
                // widget::text("Use custom window borders and close/minimize/maximize buttons").size(12),
                // widget::Space::with_height(5),

                widget::checkbox("Antialiasing (UI) - Requires Restart", config.ui_antialiasing.unwrap_or(true))
                    .on_toggle(|n| Message::LauncherSettings(
                        LauncherSettingsMessage::ToggleAntialiasing(n)
                    )),
                widget::text("Makes text/menus crisper. Also nudges the launcher into using your dedicated GPU for the User Interface").size(12).style(tsubtitle),
                                widget::Space::with_height(5),
                widget::checkbox("Discord Rich Presence - Requires Restart", config.rich_presence.unwrap_or(true))
                                    .on_toggle(|n| Message::LauncherSettings(
                                        LauncherSettingsMessage::ToggleDiscordRichPresence(n)
                                    )),
                widget::Space::with_height(5),
                widget::checkbox("Remember window size", config.window.as_ref().is_none_or(|n| n.save_window_size))
                    .on_toggle(|n| Message::LauncherSettings(LauncherSettingsMessage::ToggleWindowSize(n))),
                widget::Space::with_height(5),
                widget::checkbox("Remember last selected instance", config.persistent.clone().unwrap_or_default().selected_remembered)
                    .on_toggle(|n| Message::LauncherSettings(LauncherSettingsMessage::ToggleInstanceRemembering(n))),
            ]
            .spacing(5)
            .into(),

            widget::column![
                widget::row![
                    widget::text!("UI Idle FPS ({idle_fps})")
                        .size(15)
                        .width(SETTING_WIDTH),
                    widget::slider(2.0..=20.0, idle_fps as f64, |n| Message::LauncherSettings(
                        LauncherSettingsMessage::UiIdleFps(n)
                    ))
                    .step(1.0).shift_step(1.0),
                ]
                .align_y(Alignment::Center)
                .spacing(5),
                widget::text(r#"(Default: 6) Reduces resource usage when launcher is idle.
Only increase if progress bars stutter or "not responding" dialogs show"#).size(12).style(tsubtitle),
            ].spacing(5).into()
        ])
        .into()
    }
}

fn get_ui_opacity(config: &LauncherConfig) -> widget::Column<'static, Message, LauncherTheme> {
    let ui_opacity = config.c_ui_opacity();
    let t = |t| widget::text(t).size(12).style(tsubtitle);

    widget::column![
        widget::row![
            widget::text!("Window Opacity ({ui_opacity:.2}x)")
                .width(SETTING_WIDTH)
                .size(15),
            widget::slider(0.5..=1.0, ui_opacity, |n| Message::LauncherSettings(
                LauncherSettingsMessage::UiOpacity(n)
            ))
            .step(0.1)
        ]
        .spacing(5)
        .align_y(Alignment::Center),
        t("Window background transparency\n(May not work on all systems/GPUs)"),
        t("0.5 (translucent) ..  1.0 (opaque)"),
    ]
    .spacing(5)
}

pub fn get_theme_selector() -> widget::Row<'static, Message, LauncherTheme> {
    widget::row(LauncherThemeColor::ALL.iter().map(|color| {
        widget::button(widget::text(color.to_string()).size(14))
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
            LauncherSettingsTab::Internal => widget::column![
                widget::text("Game").size(20),
                button_with_icon(icons::folder(), "Open Launcher Folder", 16)
                    .on_press(Message::CoreOpenPath(LAUNCHER_DIR.clone())),
                widget::horizontal_rule(1),
                resolution_dialog(
                    config.global_settings.as_ref(),
                    |n| Message::LauncherSettings(
                        LauncherSettingsMessage::DefaultMinecraftWidthChanged(n)
                    ),
                    |n| Message::LauncherSettings(
                        LauncherSettingsMessage::DefaultMinecraftHeightChanged(n)
                    ),
                ),
                widget::horizontal_rule(1),
                "Global Java Arguments:",
                get_args_list(config.extra_java_args.as_deref(), |msg| {
                    Message::LauncherSettings(LauncherSettingsMessage::GlobalJavaArgs(msg))
                }),
                widget::Space::with_height(5),
                "Global Pre-Launch Prefix:",
                widget::text(PREFIX_EXPLANATION).size(12).style(tsubtitle),
                get_args_list(
                    config
                        .global_settings
                        .as_ref()
                        .and_then(|n| n.pre_launch_prefix.as_deref()),
                    |n| Message::LauncherSettings(LauncherSettingsMessage::GlobalPreLaunchPrefix(
                        n
                    )),
                ),
                args_split_by_space(menu.arg_split_by_space),
                widget::horizontal_rule(1),
                widget::row![
                    button_with_icon(icons::bin(), "Clear Java installs", 16).on_press(
                        Message::LauncherSettings(LauncherSettingsMessage::ClearJavaInstalls)
                    ),
                    widget::text(
                        "Might fix some Java problems.\nPerfectly safe, will be redownloaded."
                    )
                    .style(tsubtitle)
                    .size(12),
                ]
                .spacing(10)
                .wrap(),
            ]
            .spacing(SETTINGS_SPACING)
            .padding(16)
            .into(),
            LauncherSettingsTab::About => view_about_tab(),
        }
    }
}

fn view_about_tab() -> Element<'static> {
    let gpl3_button = widget::button(underline(
        widget::text("GNU GPLv3 License").size(12),
        Color::Light,
    ))
    .padding(0)
    .style(|n: &LauncherTheme, status| n.style_button(status, StyleButton::FlatDark))
    .on_press(Message::LicenseChangeTab(crate::state::LicenseTab::Gpl3));

    let links = widget::row![
        button_with_icon(icons::globe(), "Website", 16)
            .on_press(Message::CoreOpenLink(WEBSITE.to_owned())),
        button_with_icon(icons::github(), "Github", 16)
            .on_press(Message::CoreOpenLink(GITHUB.to_owned())),
        button_with_icon(icons::discord(), "Discord", 16)
            .on_press(Message::CoreOpenLink(DISCORD.to_owned())),
    ]
    .spacing(5)
    .wrap();

    let menus = widget::row![
        widget::button("Changelog").on_press(Message::CoreOpenChangeLog),
        widget::button("Welcome Screen").on_press(Message::CoreOpenIntro),
        widget::button("Licenses").on_press(Message::LicenseOpen),
    ]
    .spacing(5)
    .wrap();

    widget::column![
        widget::column![
            widget::text("About QuantumLauncher").size(20),
            "Copyright 2025 Mrmayman & Contributors"
        ]
        .spacing(5),
        menus,
        links,
        widget::button(widget::image(IMG_ICED.clone()).height(40))
            .on_press(Message::CoreOpenLink("https://iced.rs".to_owned()))
            .padding(5)
            .style(|n: &LauncherTheme, status| n.style_button(status, StyleButton::Flat)),
        widget::horizontal_rule(1),
        widget::column![
            widget::row![
                widget::text("QuantumLauncher is free and open source software under the ")
                    .size(12),
                gpl3_button,
            ]
            .wrap(),
            widget::text(
                r"No warranty is provided for this software.
You're free to share, modify, and redistribute it under the same license."
            )
            .size(12),
            widget::text(
                r"If you like this launcher, consider sharing it with your friends.
Every new user motivates me to keep working on this :)"
            )
            .size(12),
        ]
        .padding(iced::Padding {
            top: 10.0,
            bottom: 10.0,
            left: 15.0,
            right: 10.0,
        })
        .spacing(5),
    ]
    .padding(16)
    .spacing(SETTINGS_SPACING)
    .into()
}
