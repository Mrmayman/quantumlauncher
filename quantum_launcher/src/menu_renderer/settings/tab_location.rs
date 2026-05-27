use std::{collections::HashSet, path::Path};

use iced::{Alignment, Length, widget};
use ql_core::LAUNCHER_DIR;

use crate::{
    icons,
    menu_renderer::{Column, button_with_icon, checkered_list, tsubtitle},
    state::{LauncherSettingsMessage, MenuLauncherSettings, Message, PathKind},
    stylesheet::color::Color,
};

pub(super) fn view(menu: &MenuLauncherSettings) -> Column<'_> {
    let t = |s| widget::text(s).size(12).style(tsubtitle);

    let current_dir_text = widget::text(redact_path(&LAUNCHER_DIR))
        .size(14)
        .font(crate::menu_renderer::FONT_MONO);

    checkered_list([
        widget::column![
            widget::row![
                widget::text("Location").size(20),
                widget::horizontal_space(),
                button_with_icon(icons::folder_s(12), "Open Launcher Folder", 12)
                    .padding([5, 10])
                    .on_press(Message::CoreOpenPath(
                        std::env::current_exe()
                            .unwrap_or_default()
                            .parent()
                            .unwrap_or(Path::new(""))
                            .to_path_buf(),
                    )),
            ]
            .align_y(Alignment::Center),
        ],
        widget::column![
            widget::text("Current Data Directory").size(16),
            t("This is the active folder where all your Minecraft instances, mods, and launcher settings are saved."),
            widget::Space::with_height(5),
            widget::row![
                current_dir_text.width(Length::Fill),
                button_with_icon(icons::folder_s(12), "Open Folder", 12)
                    .padding([5, 10])
                    .on_press(Message::CoreOpenPath(LAUNCHER_DIR.clone())),
            ]
            .align_y(Alignment::Center)
            .spacing(10),
        ]
        .spacing(5),
        view_portable_mode_section(menu),
        view_system_redirect_section(menu),
    ])
}

fn view_portable_mode_section(menu: &MenuLauncherSettings) -> Column<'_> {
    let status = &menu.portable_mode_status.portable;
    let temp_path = &menu.temp_paths.portable;

    let t = |s| widget::text(s).size(12).style(tsubtitle);

    let is_active = status.is_some();

    let toggle_msg = if is_active {
        LauncherSettingsMessage::DisablePortableMode
    } else {
        LauncherSettingsMessage::EnablePortableMode
    };

    let portable_checkbox = widget::checkbox("Enable Portable Mode", is_active)
        .size(16)
        .text_size(15)
        .on_toggle(move |_| Message::LauncherSettings(toggle_msg.clone()));

    let mut col = widget::column![
        widget::row![
            widget::text("Portable Mode").size(16),
            widget::horizontal_space(),
            status_badge(
                if is_active { "ACTIVE" } else { "INACTIVE" },
                if is_active {
                    iced::Color::from_rgb8(0x94, 0xe2, 0xd5)
                } else {
                    iced::Color::from_rgb8(0xf3, 0x8b, 0xa8)
                },
            ),
        ]
        .align_y(Alignment::Center),
        t("Store launcher data alongside the executable. (Highest Priority)"),
        widget::Space::with_height(5),
        portable_checkbox,
    ]
    .spacing(5);

    if is_active {
        let (current_path, current_flags) = match status {
            Some(s) => (
                s.path
                    .as_ref()
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                &s.flags,
            ),
            _ => (String::new(), &HashSet::new()),
        };

        let has_changes =
            temp_path != &current_path || &menu.temp_paths.portable_flags != current_flags;

        let mut path_row = widget::row![
            widget::text_input(
                "Leave blank to store right next to the executable.",
                temp_path
            )
            .on_input(|s| {
                Message::LauncherSettings(LauncherSettingsMessage::SetTempPath(
                    PathKind::Portable,
                    s,
                ))
            })
            .padding(6)
            .size(13)
            .font(crate::menu_renderer::FONT_MONO)
            .width(Length::FillPortion(3)),
            button_with_icon(icons::folder_s(12), "Browse", 12)
                .padding([5, 10])
                .on_press(Message::LauncherSettings(
                    LauncherSettingsMessage::PickPortablePath,
                )),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        if has_changes {
            path_row = path_row.push(
                widget::button(widget::text("Apply Changes").size(13))
                    .padding([5, 15])
                    .on_press(Message::LauncherSettings(
                        LauncherSettingsMessage::EnablePortableMode,
                    )),
            );
        }

        let custom_path_section = widget::column![
            widget::text("Custom Storage Path (Optional)").size(14),
            t("Specify a sub-directory path relative to the executable, or an absolute path."),
            widget::Space::with_height(2),
            path_row,
        ]
        .spacing(5)
        .padding(iced::Padding {
            top: 10.0,
            right: 0.0,
            bottom: 0.0,
            left: 25.0,
        });

        col = col.push(custom_path_section);
    }

    col
}

fn view_system_redirect_section(menu: &MenuLauncherSettings) -> Column<'_> {
    let status = &menu.portable_mode_status.system_redirect;
    let portable_active = menu.portable_mode_status.portable.is_some();
    let temp_path = &menu.temp_paths.system_redirect;

    let t = |s| widget::text(s).size(12).style(tsubtitle);

    let is_active = status.is_some();

    let toggle_msg = if is_active {
        LauncherSettingsMessage::DisableSystemRedirect
    } else {
        LauncherSettingsMessage::EnableSystemRedirect
    };

    let redirect_checkbox = widget::checkbox("Enable System-Wide Redirection", is_active)
        .size(16)
        .text_size(15)
        .on_toggle(move |_| Message::LauncherSettings(toggle_msg.clone()));

    let mut status_row = widget::row![
        widget::text("System Redirection").size(16),
        widget::horizontal_space(),
    ]
    .align_y(Alignment::Center)
    .spacing(10);

    if is_active && portable_active {
        status_row = status_row.push(status_badge(
            "OVERRIDDEN",
            iced::Color::from_rgb8(0xfa, 0xa3, 0x56),
        ));
    }

    status_row = status_row.push(status_badge(
        if is_active { "ACTIVE" } else { "INACTIVE" },
        if is_active {
            iced::Color::from_rgb8(0x94, 0xe2, 0xd5)
        } else {
            iced::Color::from_rgb8(0xf3, 0x8b, 0xa8)
        },
    ));

    let mut col = widget::column![
        status_row,
        t("Redirect data globally via the system data directory (~/.local/share/QuantumLauncher)."),
    ]
    .spacing(5);

    if is_active && portable_active {
        col = col.push(
            t("Warning: Portable Mode is currently active and takes priority over this setting.")
                .style(
                    |_: &crate::stylesheet::styles::LauncherTheme| widget::text::Style {
                        color: Some(iced::Color::from_rgb8(0xfa, 0xa3, 0x56)),
                    },
                ),
        );
    }

    col = col.push(widget::Space::with_height(5));
    col = col.push(redirect_checkbox);

    if is_active {
        let (current_path, current_flags) = match status {
            Some(s) => (
                s.path
                    .as_ref()
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                &s.flags,
            ),
            _ => (String::new(), &HashSet::new()),
        };

        let has_changes =
            temp_path != &current_path || &menu.temp_paths.system_redirect_flags != current_flags;

        let mut path_row = widget::row![
            widget::text_input("Enter redirection path...", temp_path)
                .on_input(|s| {
                    Message::LauncherSettings(LauncherSettingsMessage::SetTempPath(
                        PathKind::SystemRedirect,
                        s,
                    ))
                })
                .padding(6)
                .size(13)
                .font(crate::menu_renderer::FONT_MONO)
                .width(Length::FillPortion(3)),
            button_with_icon(icons::folder_s(12), "Browse", 12)
                .padding([5, 10])
                .on_press(Message::LauncherSettings(
                    LauncherSettingsMessage::PickSystemRedirectPath,
                )),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        if has_changes {
            path_row = path_row.push(
                widget::button(widget::text("Apply Changes").size(13))
                    .padding([5, 15])
                    .on_press(Message::LauncherSettings(
                        LauncherSettingsMessage::EnableSystemRedirect,
                    )),
            );
        }

        let custom_path_section = widget::column![
            widget::text("Global Redirect Path").size(14),
            t("All instances will be redirected to this location unless a local portable override exists."),
            widget::Space::with_height(2),
            path_row,
        ]
        .spacing(5)
        .padding(iced::Padding {
            top: 10.0,
            right: 0.0,
            bottom: 0.0,
            left: 25.0,
        });

        col = col.push(custom_path_section);
    }

    col
}

fn redact_path(path: &Path) -> String {
    let path_str = path.to_string_lossy();
    if let Some(home) = dirs::home_dir() {
        let home_str = home.to_string_lossy();
        if path_str.starts_with(&*home_str) {
            return path_str.replacen(&*home_str, "~", 1);
        }
    }
    path_str.into_owned()
}

fn status_badge<'a>(
    label: &'a str,
    accent_color: iced::Color,
) -> widget::Container<'a, Message, crate::stylesheet::styles::LauncherTheme> {
    widget::container(
        widget::row![
            widget::text("●")
                .size(10)
                .style(move |_| widget::text::Style {
                    color: Some(accent_color),
                }),
            widget::text(label)
                .size(12)
                .style(
                    |t: &crate::stylesheet::styles::LauncherTheme| widget::text::Style {
                        color: Some(if t.is_light() {
                            iced::Color::BLACK
                        } else {
                            iced::Color::WHITE
                        }),
                    },
                ),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .padding([2, 8])
    .style(|t: &crate::stylesheet::styles::LauncherTheme| {
        let (bg_color, border_color, border_width) = if t.is_light() {
            (
                t.get(Color::SecondLight).scale_alpha(0.35),
                t.get(Color::Mid),
                1.5,
            )
        } else {
            (
                t.get(Color::SecondDark).scale_alpha(0.4),
                t.get(Color::SecondLight),
                1.0,
            )
        };
        widget::container::Style {
            background: Some(iced::Background::Color(bg_color)),
            border: iced::Border {
                color: border_color,
                width: border_width,
                radius: 6.0.into(),
            },
            ..Default::default()
        }
    })
}
