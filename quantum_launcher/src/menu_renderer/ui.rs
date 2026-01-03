//! Helper components for the user interface

use iced::widget;

use crate::{menu_renderer::Element, state::Message};

pub fn toggler<'a, F: Fn(bool) -> Message + 'a>(
    text: impl Into<Element<'a>>,
    enabled: bool,
    on_toggle: F,
) -> Element<'a> {
    widget::row![widget::toggler(enabled).on_toggle(on_toggle), text.into()]
        .spacing(5)
        .into()
}
