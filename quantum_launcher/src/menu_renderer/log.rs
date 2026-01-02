//! A "log viewer" made with iced widgets.
//!
//! # Features
//! - Basic "word-wrapping" (cuts off sometimes)
//! - Rough, basic scrolling (because `iced::widget::scrollable`
//!   is magic I can't replicate).
//! - Clicking to copy a line to clipboard.
//! - High performance for large logs
//!
//! # Limitations
//! - Scrolling is janky
//! - When scrolling, large lines get jumped over
//! - Overall layout and widget size is messy sometimes
//!
//! `iced::widget::scrollable` renders the whole thing, not slices,
//! so I can't use it. I can't figure out how to make this as smooth as
//! `scrollable`.
//!
//! The way this works is,
//! it assumes logs are a big list of lines.
//! This renders a "subslice" of this big list.
//!
//! For example:
//!
//! ```txt
//! [info] Starting up launcher
//! [info] Installing fabric             <--|
//! - Downloading library: (1/7)            |
//! - Downloading library: (2/7)            |
//! - Downloading library: (3/7)            |
//! - Downloading library: (4/7)            |
//! - Downloading library: (5/7)         <--|
//! - Downloading library: (6/7)
//! - Downloading library: (7/7)
//! [info] Finished installing fabric
//! ```
//!
//! You can see uses of this in the following
//! (but not limited to) places:
//! - Instance log viewer
//! - Launcher debug log (the bottom bar)
//!
//! See [`Launcher::view_launcher_log`] for more info.

use iced::{widget, Length};

use crate::{
    state::{Launcher, Message},
    stylesheet::{color::Color, styles::LauncherTheme, widgets::StyleButton},
};

use super::Element;

impl Launcher {
    /// Renders the log. See the [`crate::menu_renderer::log`] module
    /// documentation for basic context.
    ///
    /// # Arguments
    /// - `text`: A list of log lines and their type
    ///   (info/error/point)
    /// - `text_size`: The size of the characters of the log.
    ///   Recommended: `12.0`
    /// - `scroll`: The amount of lines scrolled down.
    ///   `0` for the beginning, add 1 to it as you scroll down.
    ///
    /// - `msg`: A closure returning the [`Message`] to be
    ///   called when scrolling **relative**.
    /// - `msg_absolute`: A closure returning the [`Message`] to be
    ///   called when scrolling **absolute**.
    ///
    /// Returns the `Element` containing the log viewer.
    pub fn view_launcher_log<'a, T>(
        text: Vec<T>,
        text_size: f32,
        scroll: isize,

        fn_scroll: impl Fn(isize) -> Message + Clone + 'a,
        fn_scroll_absolute: impl Fn(isize) -> Message + Clone + 'a,

        render: impl Fn(&T) -> Element<'a> + Clone + 'a,
        stringify: impl Fn(&T) -> String + Clone + 'a,
    ) -> Element<'a>
    where
        T: Clone + 'a,
    {
        widget::responsive(move |size| {
            let msg = fn_scroll.clone();
            let msg_absolute = fn_scroll_absolute.clone();
            let text = text.clone();

            let render = render.clone();
            let stringify = stringify.clone();

            let (text_len, column) =
                log_inner(&text, text_size, scroll, size.height, render, stringify);
            let text_len = text_len as f64;

            widget::mouse_area(
                widget::container(widget::row![
                    column.height(Length::Fill),
                    widget::vertical_slider(0.0..=text_len, text_len - scroll as f64, move |val| {
                        msg_absolute(text_len as isize - val as isize)
                    })
                ])
                .style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::ExtraDark)),
            )
            .on_scroll(move |n| {
                let lines = match n {
                    iced::mouse::ScrollDelta::Lines { y, .. } => y as isize,
                    iced::mouse::ScrollDelta::Pixels { y, .. } => (y / text_size) as isize,
                };
                msg(lines)
            })
            .into()
        })
        .into()
    }
}

fn log_inner<'a, T: Clone>(
    text: &[T],
    text_size: f32,
    scroll: isize,
    height_limit: f32,
    render: impl Fn(&T) -> Element<'a>,
    stringify: impl Fn(&T) -> String,
) -> (usize, widget::Column<'a, Message, LauncherTheme>) {
    let len = text.len();

    let start_pos = scroll as usize;
    let end_pos = (height_limit / (text_size * 1.7)) as usize;
    let end_pos = std::cmp::min(start_pos + end_pos, len);

    let text = if start_pos >= len {
        Vec::new()
    } else {
        text[start_pos..end_pos].to_vec()
    };
    let screen_len = text.len();

    let column = widget::column(text.into_iter().map(|msg| {
        widget::button(render(&msg))
            .padding(0)
            .style(|n: &LauncherTheme, status| n.style_button(status, StyleButton::FlatExtraDark))
            .on_press(Message::CoreCopyText(stringify(&msg)))
            .into()
    }))
    .push(widget::space().width(Length::Fill))
    .spacing(4);

    (len.checked_sub(screen_len).unwrap_or(len), column)
}
