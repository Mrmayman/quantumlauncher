use iced::{widget, Length};

use crate::{
    icon_manager,
    menu_renderer::Element,
    state::Message,
    stylesheet::{
        color::Color,
        styles::{LauncherTheme, BORDER_RADIUS, BORDER_WIDTH},
        widgets::StyleButton,
    },
};

pub fn select_box<'a>(
    e: impl Into<Element<'a>>,
    is_checked: bool,
    message: Message,
) -> widget::Button<'a, Message, LauncherTheme> {
    widget::button(underline(e, Color::Dark))
        .on_press(message)
        .style(move |t: &LauncherTheme, s| {
            t.style_button(
                s,
                if is_checked {
                    StyleButton::Flat
                } else {
                    StyleButton::FlatExtraDark
                },
            )
        })
}

pub fn link<'a>(
    e: impl Into<Element<'a>>,
    url: String,
) -> widget::Button<'a, Message, LauncherTheme> {
    widget::button(underline(e, Color::Light))
        .on_press(Message::CoreOpenLink(url))
        .padding(0)
        .style(|n: &LauncherTheme, status| n.style_button(status, StyleButton::FlatDark))
}

pub fn underline<'a>(
    e: impl Into<Element<'a>>,
    color: Color,
) -> widget::Stack<'a, Message, LauncherTheme> {
    widget::stack!(
        widget::column![e.into()],
        widget::column![
            widget::vertical_space(),
            widget::horizontal_rule(1).style(move |t: &LauncherTheme| t.style_rule(color, 1)),
            widget::Space::with_height(0.8),
        ]
    )
}

pub fn center_x<'a>(e: impl Into<Element<'a>>) -> Element<'a> {
    widget::row![
        widget::horizontal_space(),
        e.into(),
        widget::horizontal_space(),
    ]
    .into()
}

pub fn tooltip<'a>(
    e: impl Into<Element<'a>>,
    tooltip: impl Into<Element<'a>>,
    position: widget::tooltip::Position,
) -> widget::Tooltip<'a, Message, LauncherTheme> {
    widget::tooltip(e, tooltip, position)
        .style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::ExtraDark))
}

pub fn back_button<'a>() -> widget::Button<'a, Message, LauncherTheme> {
    button_with_icon(icon_manager::back_with_size(14), "Back", 14)
}

pub fn ctxbox<'a>(inner: impl Into<Element<'a>>) -> widget::Container<'a, Message, LauncherTheme> {
    widget::container(inner)
        .padding(10)
        .style(|t: &LauncherTheme| {
            t.style_container_round_box(BORDER_WIDTH, Color::Dark, BORDER_RADIUS)
        })
}

pub fn subbutton_with_icon<'a>(
    icon: impl Into<Element<'a>>,
    text: &'a str,
) -> widget::Button<'a, Message, LauncherTheme> {
    widget::button(
        widget::row![icon.into(), widget::text(text).size(12)]
            .align_y(iced::alignment::Vertical::Center)
            .spacing(8)
            .padding(1),
    )
    .style(|t: &LauncherTheme, s| {
        t.style_button(s, crate::stylesheet::widgets::StyleButton::RoundDark)
    })
}

pub fn button_with_icon<'a>(
    icon: impl Into<Element<'a>>,
    text: &'a str,
    size: u16,
) -> widget::Button<'a, Message, LauncherTheme> {
    widget::button(
        widget::row![icon.into(), widget::text(text).size(size)]
            .align_y(iced::alignment::Vertical::Center)
            .spacing(10)
            .padding(3),
    )
}

pub fn shortcut_ctrl<'a>(key: &str) -> Element<'a> {
    #[cfg(target_os = "macos")]
    return widget::text!("Command + {key}").size(12).into();

    widget::text!("Control + {key}").size(12).into()
}

pub fn sidebar_button<'a, A: PartialEq>(
    current: &A,
    selected: &A,
    text: impl Into<Element<'a>>,
    message: Message,
) -> Element<'a> {
    if current == selected {
        widget::container(widget::row!(widget::Space::with_width(5), text.into()))
            .style(LauncherTheme::style_container_selected_flat_button)
            .width(Length::Fill)
            .padding(5)
            .into()
    } else {
        widget::button(text)
            .on_press(message)
            .style(|n: &LauncherTheme, status| n.style_button(status, StyleButton::FlatExtraDark))
            .width(Length::Fill)
            .into()
    }
}
