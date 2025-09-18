use iced::{widget, Length};
use ql_core::{LAUNCHER_DIR, WEBSITE};

use super::{
    back_button, button_with_icon, get_theme_selector, sidebar_button, underline, Element, DISCORD,
    GITHUB,
};
use crate::menu_renderer::edit_instance::{
    global_java_args_dialog, global_pre_launch_prefix_dialog, resolution_dialog,
};
use crate::{
    config::LauncherConfig,
    icon_manager,
    state::{LauncherSettingsMessage, LauncherSettingsTab, MenuLauncherSettings, Message},
    stylesheet::{
        color::Color,
        styles::{LauncherTheme, LauncherThemeColor},
        widgets::StyleButton,
    },
};

const SETTINGS_SPACING: f32 = 7.0;
const PADDING_NOT_BOTTOM: iced::Padding = iced::Padding {
    top: 10.0,
    bottom: 0.0,
    left: 10.0,
    right: 10.0,
};
const PADDING_LEFT: iced::Padding = iced::Padding {
    top: 0.0,
    right: 0.0,
    bottom: 0.0,
    left: 10.0,
};

impl MenuLauncherSettings {
    pub fn view<'a>(&'a self, config: &'a LauncherConfig, window_size: (f32, f32)) -> Element<'a> {
        widget::row![
            widget::container(
                widget::column![
                    widget::column!(back_button().on_press(Message::LaunchScreenOpen {
                        message: None,
                        clear_selection: false
                    }))
                    .padding(PADDING_NOT_BOTTOM),
                    widget::row![
                        icon_manager::settings_with_size(20),
                        widget::text("Settings").size(20),
                    ]
                    .padding(iced::Padding {
                        top: 5.0,
                        right: 0.0,
                        bottom: 2.0,
                        left: 10.0,
                    })
                    .spacing(10),
                    widget::column(LauncherSettingsTab::ALL.iter().map(|tab| {
                        let text = widget::text(tab.to_string());
                        sidebar_button(
                            tab,
                            &self.selected_tab,
                            text,
                            Message::LauncherSettings(LauncherSettingsMessage::ChangeTab(*tab)),
                        )
                    }))
                ]
                .spacing(10)
            )
            .height(Length::Fill)
            .width(180)
            .style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::ExtraDark)),
            widget::scrollable(self.selected_tab.view(config, self, window_size))
                .width(Length::Fill)
                .style(LauncherTheme::style_scrollable_flat_dark)
        ]
        .into()
    }

    fn view_ui_tab<'a>(&'a self, config: &'a LauncherConfig) -> Element<'a> {
        let (light, dark) = get_theme_selector(config);

        let color_scheme_picker = LauncherThemeColor::ALL.iter().map(|color| {
            widget::button(widget::text(color.to_string()).size(14))
                .style(|theme: &LauncherTheme, s| {
                    LauncherTheme {
                        lightness: theme.lightness,
                        color: *color,
                    }
                    .style_button(s, StyleButton::Round)
                })
                .on_press(Message::LauncherSettings(
                    LauncherSettingsMessage::ColorSchemePicked(color.to_string()),
                ))
                .into()
        });

        widget::column!(
            widget::column![widget::text("User Interface").size(20)].padding(PADDING_NOT_BOTTOM),
            widget::column!("Theme:", widget::row![light, dark].spacing(5))
                .padding(iced::Padding {
                    top: 0.0,
                    bottom: 10.0,
                    left: 10.0,
                    right: 10.0,
                })
                .spacing(5),
            widget::horizontal_rule(1),
            widget::column!(
                "Color scheme:",
                widget::row(color_scheme_picker).spacing(5).wrap()
            )
            .padding(10)
            .spacing(5),
            widget::horizontal_rule(1),
            widget::column![
                widget::row![widget::text!("UI Scale ({:.2}x)  ", self.temp_scale)]
                    .push_maybe(
                        ((self.temp_scale - config.ui_scale.unwrap_or(1.0)).abs() > 0.01)
                            .then_some(
                                widget::button(widget::text("Apply").size(12))
                                    .padding(iced::Padding {
                                        top: 2.0,
                                        bottom: 2.0,
                                        right: 5.0,
                                        left: 5.0,
                                    })
                                    .on_press(Message::LauncherSettings(
                                        LauncherSettingsMessage::UiScaleApply
                                    ))
                            )
                    )
                    .align_y(iced::Alignment::Center),
                widget::slider(0.5..=2.0, self.temp_scale, |n| Message::LauncherSettings(
                    LauncherSettingsMessage::UiScale(n)
                ))
                .step(0.1),
                widget::text("Warning: slightly buggy").size(12),
            ]
            .padding(10)
            .spacing(5),
            widget::horizontal_rule(1),
            widget::column![
                widget::checkbox("Antialiasing (UI)", config.antialiasing.unwrap_or(true))
                    .on_toggle(|n| Message::LauncherSettings(
                        LauncherSettingsMessage::ToggleAntialiasing(n)
                    )),
                widget::text("Makes text/menus crisper. Also nudges the launcher into using your dedicated GPU for the User Interface.\nRequires restarting the launcher.").size(12),
                widget::Space::with_height(5),
                widget::checkbox("Remember window size", config.window.as_ref().is_none_or(|n| n.save_window_size))
                    .on_toggle(|n| Message::LauncherSettings(LauncherSettingsMessage::ToggleWindowSize(n))),
                widget::text("If enabled, the launcher window will retain its size from the last session.").size(12),
            ]
            .padding(10)
            .spacing(5)
        )
        .spacing(SETTINGS_SPACING)
        .into()
    }
}

impl LauncherSettingsTab {
    pub fn view<'a>(
        &'a self,
        config: &'a LauncherConfig,
        menu: &'a MenuLauncherSettings,
        window_size: (f32, f32),
    ) -> Element<'a> {
        match self {
            LauncherSettingsTab::UserInterface => menu.view_ui_tab(config),
            LauncherSettingsTab::Internal => widget::column![
                widget::column![
                    widget::text("Game").size(20),
                    button_with_icon(icon_manager::folder(), "Open Launcher Folder", 16)
                        .on_press(Message::CoreOpenPath(LAUNCHER_DIR.clone()))
                ]
                .spacing(10)
                .padding(10),
                widget::horizontal_rule(1),
                widget::column![resolution_dialog(
                    config.global_settings.as_ref(),
                    |n| Message::LauncherSettings(
                        LauncherSettingsMessage::DefaultMinecraftWidthChanged(n)
                    ),
                    |n| Message::LauncherSettings(
                        LauncherSettingsMessage::DefaultMinecraftHeightChanged(n)
                    ),
                    true
                )]
                .padding(10)
                .spacing(10),
                widget::horizontal_rule(1),
                widget::column![global_java_args_dialog(
                    config.extra_java_args.as_deref(),
                    Message::LauncherSettings(LauncherSettingsMessage::GlobalJavaArgsAdd),
                    |idx| Message::LauncherSettings(LauncherSettingsMessage::GlobalJavaArgDelete(
                        idx
                    )),
                    &|arg, idx| Message::LauncherSettings(
                        LauncherSettingsMessage::GlobalJavaArgEdit(arg, idx)
                    ),
                    |idx| Message::LauncherSettings(LauncherSettingsMessage::GlobalJavaArgShiftUp(
                        idx
                    )),
                    |idx| Message::LauncherSettings(
                        LauncherSettingsMessage::GlobalJavaArgShiftDown(idx)
                    ),
                )]
                .padding(10)
                .spacing(10),
                widget::horizontal_rule(1),
                widget::column![global_pre_launch_prefix_dialog(
                    config
                        .global_settings
                        .as_ref()
                        .and_then(|n| n.pre_launch_prefix.as_deref()),
                    Message::LauncherSettings(LauncherSettingsMessage::GlobalPreLaunchPrefixAdd),
                    |idx| Message::LauncherSettings(
                        LauncherSettingsMessage::GlobalPreLaunchPrefixDelete(idx)
                    ),
                    &|arg, idx| Message::LauncherSettings(
                        LauncherSettingsMessage::GlobalPreLaunchPrefixEdit(arg, idx)
                    ),
                    |idx| Message::LauncherSettings(
                        LauncherSettingsMessage::GlobalPreLaunchPrefixShiftUp(idx)
                    ),
                    |idx| Message::LauncherSettings(
                        LauncherSettingsMessage::GlobalPreLaunchPrefixShiftDown(idx)
                    ),
                )]
                .padding(10)
                .spacing(10),
                widget::horizontal_rule(1),
                widget::column![
                    button_with_icon(icon_manager::delete(), "Clear Java installs", 16).on_press(
                        Message::LauncherSettings(LauncherSettingsMessage::ClearJavaInstalls)
                    ),
                    widget::text(
                        "Might fix some Java problems.\nPerfectly safe, will be redownloaded."
                    )
                    .size(12),
                ]
                .padding(10)
                .spacing(10),
            ]
            .spacing(SETTINGS_SPACING)
            .into(),
            LauncherSettingsTab::About => {
                let gpl3_button =
                    // widget::button(widget::rich_text![widget::span("GNU GPLv3 License").underline(true)].size(12))

                    // An Iced bug (or maybe some dumb mistake I made),
                    // putting underlines in buttons the "official" way makes them unclickable.

                    widget::button(underline(widget::text("GNU GPLv3 License").size(12), Color::Light))
                        .padding(0)
                        .style(|n: &LauncherTheme, status| n.style_button(status, StyleButton::FlatDark))
                        .on_press(Message::LicenseChangeTab(crate::state::LicenseTab::Gpl3));

                let links = widget::row![
                    button_with_icon(icon_manager::globe(), "Website", 16)
                        .on_press(Message::CoreOpenLink(WEBSITE.to_owned())),
                    button_with_icon(icon_manager::github(), "Github", 16)
                        .on_press(Message::CoreOpenLink(GITHUB.to_owned())),
                    button_with_icon(icon_manager::discord(), "Discord", 16)
                        .on_press(Message::CoreOpenLink(DISCORD.to_owned())),
                ]
                .padding(iced::Padding {
                    top: 0.0,
                    right: 0.0,
                    bottom: 10.0,
                    left: 10.0,
                })
                .spacing(5)
                .wrap();

                let menus = widget::row![
                    widget::button("Changelog").on_press(Message::CoreOpenChangeLog),
                    widget::button("Welcome Screen").on_press(Message::CoreOpenIntro),
                    widget::button("Licenses").on_press(Message::LicenseOpen),
                ]
                .padding(PADDING_LEFT)
                .spacing(5)
                .wrap();

                widget::column![
                    widget::column![
                        widget::text("About QuantumLauncher").size(20),
                        "Copyright 2025 Mrmayman & Contributors"
                    ]
                    .spacing(5)
                    .padding(PADDING_NOT_BOTTOM),
                    menus,
                    links,
                    widget::column![
                        "Made with:",
                        widget::button(widget::iced(window_size.1 / 12.0))
                            .on_press(Message::CoreOpenLink("https://iced.rs".to_owned()))
                            .padding(5)
                            .style(|n: &LauncherTheme, status| n
                                .style_button(status, StyleButton::Flat))
                    ]
                    .padding(10)
                    .spacing(5),
                    widget::horizontal_rule(1),
                    widget::column![
                        widget::row![
                            widget::text(
                                "QuantumLauncher is free and open source software under the "
                            )
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
                .spacing(SETTINGS_SPACING)
                .into()
            }
        }
    }
}
