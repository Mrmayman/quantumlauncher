use iced::widget;
use ql_core::WEBSITE;

use crate::{
    icons,
    menu_renderer::{
        Column, DISCORD, GITHUB, MATRIX, button_with_icon, settings::IMG_ICED, underline,
    },
    state::Message,
    stylesheet::{color::Color, styles::LauncherTheme, widgets::StyleButton},
};

pub(super) fn view() -> Column<'static> {
    let gpl3_button = widget::button(underline(
        widget::text("GNU GPLv3 License").size(12),
        Color::Light,
    ))
    .padding(0)
    .style(|n: &LauncherTheme, status| n.style_button(status, StyleButton::FlatDark))
    .on_press(Message::LicenseChangeTab(crate::state::LicenseTab::Gpl3));

    let links = widget::row![
        button_with_icon(icons::globe_s(12), "Website", 12)
            .padding([5, 10])
            .on_press(Message::CoreOpenLink(WEBSITE.to_owned())),
        button_with_icon(icons::github_s(12), "Github", 12)
            .padding([5, 10])
            .on_press(Message::CoreOpenLink(GITHUB.to_owned())),
        button_with_icon(icons::discord_s(12), "Discord", 12)
            .padding([5, 10])
            .on_press(Message::CoreOpenLink(DISCORD.to_owned())),
        button_with_icon(icons::chatbox_s(12), "Matrix", 12)
            .padding([5, 10])
            .on_press(Message::CoreOpenLink(MATRIX.to_owned())),
    ]
    .spacing(5)
    .wrap();

    let menus = widget::row![
        widget::button(widget::text("Changelog").size(14)).on_press(Message::CoreOpenChangeLog),
        widget::button(widget::text("Welcome Screen").size(14)).on_press(Message::CoreOpenIntro),
        widget::button(widget::text("Licenses").size(14)).on_press(Message::LicenseOpen),
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
    .spacing(10)
}
