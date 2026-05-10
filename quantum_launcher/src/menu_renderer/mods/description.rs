use frostmark::{MarkState, MarkWidget};
use iced::{
    Alignment, Length,
    widget::{self, column, row, text::Wrapping},
};
use ql_mod_manager::store::{SearchMod, StoreBackendType};

use crate::{
    icons,
    menu_renderer::{
        Element, FONT_DEFAULT, FONT_MONO, barthin, button_with_icon, tooltip, tsubtitle, underline,
    },
    state::{ImageState, InstallModsMessage, ManageModsMessage, MenuModDescription, Message},
    stylesheet::{color::Color, styles::LauncherTheme, widgets::StyleButton},
};

impl MenuModDescription {
    pub fn view<'a>(&'a self, images: &'a ImageState, tick_timer: usize) -> Element<'a> {
        let Some(details) = &self.details else {
            let dots = ".".repeat((tick_timer % 3) + 1);
            return column![widget::text!("Loading{dots}")].padding(10).into();
        };

        view_project_description(
            self.description.as_ref(),
            self.mod_id.get_backend(),
            ManageModsMessage::Open,
            details,
            images,
            tick_timer,
        )
    }
}

/// Renders the mod description page
pub fn view_project_description<'a, T: iced::advanced::text::IntoFragment<'a>>(
    description: Result<&'a Option<MarkState>, T>,
    backend: StoreBackendType,
    back_msg: impl Into<Message>,
    hit: &'a SearchMod,
    images: &'a ImageState,
    tick_timer: usize,
) -> Element<'a> {
    // Parses the Markdown description of the mod.
    let markdown_description: Element = match description {
        Ok(Some(desc)) => MarkWidget::new(desc)
            .on_clicking_link(Message::CoreOpenLink)
            .on_drawing_image(|img| images.view(Some(img.url), img.width, img.height))
            .on_updating_state(|n| InstallModsMessage::TickDesc(n).into())
            .font(FONT_DEFAULT)
            .font_mono(FONT_MONO)
            .into(),
        Ok(None) => {
            let dots = ".".repeat((tick_timer % 3) + 1);
            widget::text!("Loading{dots}").into()
        }
        Err(err) => widget::container(
            column![
                widget::text("Failed to load description").size(16),
                widget::text(err).size(13)
            ]
            .spacing(5)
            .padding(10),
        )
        .into(),
    };

    let url = format!(
        "{}{}/{}",
        match backend {
            StoreBackendType::Modrinth => "https://modrinth.com/",
            StoreBackendType::Curseforge => "https://www.curseforge.com/minecraft/",
        },
        hit.project_type,
        hit.internal_name
    );

    let top_bar = widget::container(
        row![
            button_with_icon(icons::back_s(12), "Back", 13)
                .padding([5, 8])
                .on_press(back_msg.into()),
            widget::Space::with_width(0),
            images.view(hit.icon_url.as_deref(), Some(20.0), Some(20.0)),
            widget::text(&hit.title)
                .shaping(widget::text::Shaping::Advanced)
                .width(Length::Fill)
                .size(16),
            widget::tooltip(
                button_with_icon(icons::globe_s(12), "Open Mod Page", 13)
                    .padding([5, 8])
                    .on_press(Message::CoreOpenLink(url.clone())),
                widget::text(url),
                widget::tooltip::Position::Bottom
            )
            .style(|n| n.style_container_sharp_box(0.0, Color::ExtraDark)),
            widget::button(widget::text("Copy ID").size(13).wrapping(Wrapping::None))
                .padding([5, 8])
                .on_press(Message::CoreCopyText(hit.id.clone())),
        ]
        .align_y(Alignment::Center)
        .spacing(10),
    )
    .style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::ExtraDark))
    .padding([5, 10]);

    let scroll = |e, p| {
        widget::scrollable(e)
            .width(Length::FillPortion(p))
            .height(Length::Fill)
    };

    let side_description = scroll(column![markdown_description].padding(20), 2)
        .style(LauncherTheme::style_scrollable_flat_dark);

    let side_extra_info = scroll(
        column![
            widget::text(&hit.description)
                .size(14)
                .shaping(widget::text::Shaping::Advanced),
            widget::horizontal_rule(1).style(barthin),
            // Note: When upgrading to iced 0.14, make sure to update link click handling
            widget::column(hit.urls.iter().map(|(kind, url)| {
                tooltip(
                    widget::button(underline(
                        widget::text!("{kind} →").size(13),
                        Color::SecondLight,
                    ))
                    .padding(0)
                    .style(|n: &LauncherTheme, status| {
                        n.style_button(status, StyleButton::FlatExtraDark)
                    })
                    .on_press_with(|| Message::CoreOpenLink(url.clone())),
                    widget::text(url).size(12),
                    widget::tooltip::Position::Left,
                )
                .into()
            }))
            .spacing(5),
        ]
        .push_maybe((!hit.gallery.is_empty()).then(|| {
            column![
                widget::horizontal_rule(1).style(barthin),
                widget::text("Gallery").size(20),
                widget::text("Hover to enlarge").size(12).style(tsubtitle),
                widget::column(hit.gallery.iter().map(|n| {
                    let img = || images.view(Some(&n.url), None, None);
                    column![widget::tooltip(
                        img(),
                        img(),
                        widget::tooltip::Position::Left
                    )]
                    .push_maybe(n.title.as_deref().map(|n| widget::text(n).size(14)))
                    .push_maybe(
                        n.description
                            .as_deref()
                            .map(|n| widget::text(n).size(12).style(tsubtitle)),
                    )
                    .spacing(5)
                    .into()
                }))
                .spacing(20),
            ]
            .spacing(10)
        }))
        .spacing(10)
        .padding(20)
        .width(Length::FillPortion(1)),
        1,
    )
    .style(LauncherTheme::style_scrollable_flat_extra_dark);

    column![
        top_bar,
        widget::horizontal_rule(1).style(barthin),
        row![side_description, side_extra_info]
    ]
    .into()
}
