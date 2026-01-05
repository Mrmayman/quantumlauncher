use iced::{widget, Alignment, Length};

use crate::{
    icons,
    menu_renderer::Element,
    state::Message,
    stylesheet::{
        color::Color,
        styles::{LauncherTheme, BORDER_RADIUS, BORDER_WIDTH},
        widgets::StyleButton,
    },
};

pub fn checkered_list<'a, Item: Into<Element<'a>>>(
    children: impl IntoIterator<Item = Item>,
) -> widget::Column<'a, Message, LauncherTheme> {
    widget::column(children.into_iter().enumerate().map(|(i, e)| {
        widget::container(e)
            .width(Length::Fill)
            .padding(16)
            .style(move |t: &LauncherTheme| {
                t.style_container_sharp_box(
                    0.0,
                    if i % 2 == 0 {
                        Color::Dark
                    } else {
                        Color::ExtraDark
                    },
                )
            })
            .into()
    }))
}

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
            widget::Space::with_height(1),
        ]
    )
}

pub fn underline_maybe<'a>(e: impl Into<Element<'a>>, color: Color, un: bool) -> Element<'a> {
    if un {
        underline(e, color).into()
    } else {
        e.into()
    }
}

pub fn center_x<'a>(e: impl Into<Element<'a>>) -> widget::Row<'a, Message, LauncherTheme> {
    widget::row![
        widget::horizontal_space(),
        e.into(),
        widget::horizontal_space(),
    ]
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
    button_with_icon(icons::back_s(14), "Back", 14)
}

pub fn ctxbox<'a>(inner: impl Into<Element<'a>>) -> widget::Container<'a, Message, LauncherTheme> {
    widget::container(widget::mouse_area(inner).on_press(Message::Nothing))
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
        widget::row![icon.into()]
            .push_maybe((!text.is_empty()).then_some(widget::text(text).size(12)))
            .align_y(Alignment::Center)
            .spacing(8)
            .padding(1),
    )
    .style(|t: &LauncherTheme, s| t.style_button(s, StyleButton::RoundDark))
}

pub fn button_with_icon<'a>(
    icon: impl Into<Element<'a>>,
    text: &'a str,
    size: u16,
) -> widget::Button<'a, Message, LauncherTheme> {
    widget::button(
        widget::row![icon.into()]
            .push_maybe((!text.is_empty()).then_some(widget::text(text).size(size)))
            .align_y(Alignment::Center)
            .spacing(size as f32 / 1.6),
    )
    .padding([7, 13])
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
    let is_selected = current == selected;
    let button = widget::button(text)
        .on_press_maybe((!is_selected).then_some(message))
        .style(|n: &LauncherTheme, status| n.style_button(status, StyleButton::FlatExtraDark))
        .width(Length::Fill);

    underline_maybe(button, Color::SecondDark, !is_selected)
}
