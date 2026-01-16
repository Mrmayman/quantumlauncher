//! Helper components for the user interface

use iced::{widget, Alignment};

use crate::{menu_renderer::Element, state::Message};

pub fn toggler<'a, F: Fn(bool) -> Message + 'a>(
    text: impl Into<Element<'a>>,
    enabled: bool,
    on_toggle: F,
) -> Element<'a> {
    widget::row![widget::toggler(enabled).on_toggle(on_toggle), text.into()]
        .align_y(Alignment::Center)
        .spacing(10)
        .into()
}

pub fn checkbox<'a, F: Fn(bool) -> Message + 'a>(
    text: impl Into<Element<'a>>,
    enabled: bool,
    on_toggle: F,
) -> Element<'a> {
    widget::row![widget::checkbox(enabled).on_toggle(on_toggle), text.into()]
        .align_y(Alignment::Center)
        .spacing(5)
        .into()
}
