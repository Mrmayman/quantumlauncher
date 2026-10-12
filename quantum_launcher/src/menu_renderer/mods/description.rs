use frostmark::{MarkState, MarkWidget};
use iced::{
    Alignment, Length,
    widget::{self, column, row, text::Wrapping},
};
use ql_mod_manager::store::{ModVersionInfo, SearchMod, StoreBackendType};

use crate::{
    icons,
    menu_renderer::{
        Element, FONT_DEFAULT, FONT_MONO, barthin, button_with_icon, tooltip, tsubtitle, underline,
    },
    state::{
        ImageState, InstallModsMessage, ManageModsMessage, MenuModDescription, Message,
        ModDescriptionMessage,
    },
    stylesheet::{color::Color, styles::LauncherTheme, widgets::StyleButton},
};

impl MenuModDescription {
    pub fn view<'a>(&'a self, images: &'a ImageState, tick_timer: usize) -> Element<'a> {
        let Some(details) = &self.details else {
            let dots = ".".repeat((tick_timer % 3) + 1);
            return column![widget::text!("Loading{dots}")].padding(10).into();
        };

        view_project_description(ProjectDescriptionArgs {
            description: self.description.as_ref(),
            backend: self.mod_id.get_backend(),
            back_msg: ManageModsMessage::Open.into(),
            hit: details,
            images,
            tick_timer,
            versions: self.versions.as_ref(),
            selected_version: self.selected_version.as_deref(),
            version_game_filter: self.version_game_filter.as_deref(),
            version_loader_filter: self.version_loader_filter.as_deref(),
            versions_only: self.show_all_versions,
            version_msg: |version| ModDescriptionMessage::SelectVersion(version).into(),
            download_version_msg: |version| ModDescriptionMessage::DownloadVersion(version).into(),
            game_filter_msg: |filter| ModDescriptionMessage::SetVersionGameFilter(filter).into(),
            loader_filter_msg: |filter| {
                ModDescriptionMessage::SetVersionLoaderFilter(filter).into()
            },
            download_msg: Some(ModDescriptionMessage::DownloadSelectedVersion.into()),
            show_all_msg: Some(ModDescriptionMessage::ShowAllVersions.into()),
            versions_back_msg: Some(ModDescriptionMessage::BackFromVersions.into()),
        })
    }
}

pub struct ProjectDescriptionArgs<'a, T> {
    pub description: Result<&'a Option<MarkState>, T>,
    pub backend: StoreBackendType,
    pub back_msg: Message,
    pub hit: &'a SearchMod,
    pub images: &'a ImageState,
    pub tick_timer: usize,
    pub versions: Option<&'a Result<Vec<ModVersionInfo>, String>>,
    pub selected_version: Option<&'a str>,
    pub version_game_filter: Option<&'a str>,
    pub version_loader_filter: Option<&'a str>,
    pub versions_only: bool,
    pub version_msg: fn(String) -> Message,
    pub download_version_msg: fn(String) -> Message,
    pub game_filter_msg: fn(Option<String>) -> Message,
    pub loader_filter_msg: fn(Option<String>) -> Message,
    pub download_msg: Option<Message>,
    pub show_all_msg: Option<Message>,
    pub versions_back_msg: Option<Message>,
}

/// Renders the mod description page
pub fn view_project_description<'a, T: iced::advanced::text::IntoFragment<'a>>(
    args: ProjectDescriptionArgs<'a, T>,
) -> Element<'a> {
    let ProjectDescriptionArgs {
        description,
        backend,
        back_msg,
        hit,
        images,
        tick_timer,
        versions,
        selected_version,
        version_game_filter,
        version_loader_filter,
        versions_only,
        version_msg,
        download_version_msg,
        game_filter_msg,
        loader_filter_msg,
        download_msg: _download_msg,
        show_all_msg,
        versions_back_msg,
    } = args;
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
                .on_press(if versions_only {
                    versions_back_msg.unwrap_or(back_msg)
                } else {
                    back_msg
                }),
            widget::Space::with_width(0),
            images.view(hit.icon_url.as_deref(), Some(20.0), Some(20.0)),
            widget::text(&*hit.title)
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
                .on_press_with(|| Message::CoreCopyText(hit.id.to_string())),
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

    let version_view: Element = match versions {
        Some(Ok(items)) => {
            let mut game_options = vec!["All".to_owned()];
            let mut loader_options = vec!["All".to_owned()];
            for item in items {
                for game_version in &item.game_versions {
                    if !game_options.contains(game_version) {
                        game_options.push(game_version.clone());
                    }
                }
                for loader in &item.loaders {
                    if !loader_options.contains(loader) {
                        loader_options.push(loader.clone());
                    }
                }
            }

            let game_filter = version_game_filter.unwrap_or("All");
            let loader_filter = version_loader_filter.unwrap_or("All");
            let has_loader_data = loader_options.len() > 1;
            let filtered_items = items.iter().filter(|item| {
                (game_filter == "All"
                    || item
                        .game_versions
                        .iter()
                        .any(|version| version == game_filter))
                    && (!has_loader_data
                        || loader_filter == "All"
                        || item.loaders.iter().any(|loader| loader == loader_filter))
            });

            let header = widget::container(
                row![
                    widget::text("Version").width(Length::FillPortion(3)),
                    widget::text("Minecraft").width(Length::FillPortion(2)),
                    widget::text("Loader").width(Length::FillPortion(2)),
                    widget::text("Published").width(Length::FillPortion(2)),
                    widget::Space::with_width(90)
                ]
                .spacing(8),
            )
            .padding([8, 10])
            .style(|theme: &LauncherTheme| theme.style_container_sharp_box(0.0, Color::ExtraDark));

            let rows = widget::column(filtered_items.map(|item| {
                let selected = selected_version == Some(item.id.as_ref());
                let compatibility = if item.compatible {
                    widget::text("Compatible")
                        .style(|theme: &LauncherTheme| theme.style_text(Color::SecondLight))
                } else {
                    widget::text("Incompatible")
                        .style(|theme: &LauncherTheme| theme.style_text(Color::SecondDark))
                };
                let row_content = row![
                    widget::button(widget::text(if selected {
                        format!("✓ {}", item.name)
                    } else {
                        item.name.clone()
                    }))
                    .width(Length::FillPortion(3))
                    .padding([7, 8])
                    .style(move |theme: &LauncherTheme, status| {
                        theme.style_button(
                            status,
                            if selected {
                                StyleButton::SemiDarkBorder([true; 4])
                            } else {
                                StyleButton::FlatDark
                            },
                        )
                    })
                    .on_press(version_msg(item.id.to_string())),
                    widget::text(if item.game_versions.is_empty() {
                        "—".to_owned()
                    } else {
                        item.game_versions.join(", ")
                    })
                    .width(Length::FillPortion(2)),
                    widget::text(if item.loaders.is_empty() {
                        "—".to_owned()
                    } else {
                        item.loaders.join(", ")
                    })
                    .width(Length::FillPortion(2)),
                    widget::column![
                        widget::text(item.date_published.format("%Y-%m-%d").to_string()),
                        compatibility
                    ]
                    .width(Length::FillPortion(2)),
                    widget::button(widget::text("Download").size(12))
                        .padding([6, 8])
                        .on_press(download_version_msg(item.id.to_string()))
                ]
                .align_y(Alignment::Center)
                .spacing(8)
                .padding([3, 0]);

                widget::container(row_content)
                    .width(Length::Fill)
                    .padding([2, 0])
                    .into()
            }))
            .width(Length::Fill)
            .spacing(2);

            let show_all = (!versions_only && items.len() >= 100).then(|| {
                widget::button(widget::text("Show all versions").size(13))
                    .on_press(show_all_msg.clone().expect("show all message"))
            });

            let filter_bar = row![
                widget::text("Minecraft:").size(12),
                widget::pick_list(game_options, Some(game_filter.to_owned()), move |value| {
                    game_filter_msg((value != "All").then_some(value))
                }),
                widget::text("Loader:").size(12),
                widget::pick_list(
                    loader_options,
                    Some(loader_filter.to_owned()),
                    move |value| loader_filter_msg((value != "All").then_some(value))
                )
            ]
            .spacing(8)
            .align_y(Alignment::Center);

            column![
                widget::horizontal_rule(1).style(barthin),
                widget::text("Versions").size(20),
                filter_bar,
                header,
                rows
            ]
            .push_maybe(show_all)
            .spacing(5)
            .into()
        }
        Some(Err(err)) => widget::text(err).size(12).into(),
        None => widget::text("Loading versions...")
            .size(12)
            .style(tsubtitle)
            .into(),
    };

    if versions_only {
        return column![
            top_bar,
            widget::horizontal_rule(1),
            widget::container(widget::scrollable(version_view).height(Length::Fill))
                .padding(20)
                .width(Length::Fill)
                .height(Length::Fill)
        ]
        .into();
    }
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
        .push(version_view)
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
