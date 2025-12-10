use std::collections::HashSet;

use iced::{
    widget::{self, column, row, tooltip::Position},
    Alignment, Length,
};
use ql_core::{ListEntry, ListEntryKind};

use crate::{
    icon_manager,
    menu_renderer::{
        sidebar, tsubtitle,
        ui::{back_button, button_with_icon, ctxbox, sidebar_button, tooltip},
        Element,
    },
    state::{CreateInstanceMessage, MenuCreateInstance, Message},
    stylesheet::{color::Color, styles::LauncherTheme, widgets::StyleButton},
};

impl MenuCreateInstance {
    pub fn view<'a>(
        &'a self,
        existing_instances: Option<&[String]>,
        versions: Option<&'a [ListEntry]>,
    ) -> Element<'a> {
        match self {
            MenuCreateInstance::LoadingList { .. } => column![
                back_button().on_press(Message::CreateInstance(CreateInstanceMessage::Cancel)),
                widget::text("Loading version list...").size(20),
            ]
            .padding(10)
            .spacing(10)
            .into(),
            MenuCreateInstance::Choosing {
                instance_name,
                selected_version,
                download_assets,
                search_box,
                is_server,
                show_category_dropdown,
                selected_categories,
            } => {
                let pb = iced::Padding::new(7.0).left(10.0).right(10.0);
                let opened_controls = *show_category_dropdown;
                let header = column![
                    row![
                        back_button()
                            .style(|t: &LauncherTheme, s| t.style_button(s, StyleButton::RoundDark))
                            .on_press(Message::LaunchScreenOpen {
                                message: None,
                                clear_selection: false,
                                is_server: Some(*is_server),
                            }),
                        widget::button(icon_manager::filter())
                            .padding(pb)
                            .style(move |t: &LauncherTheme, s| t.style_button(
                                s,
                                if opened_controls {
                                    StyleButton::Round
                                } else {
                                    StyleButton::RoundDark
                                }
                            ))
                            .on_press(Message::CreateInstance(
                                CreateInstanceMessage::ContextMenuToggle
                            ))
                    ]
                    .spacing(5),
                    widget::text_input("Search...", search_box)
                        .size(14)
                        .on_input(|t| {
                            Message::CreateInstance(CreateInstanceMessage::SearchInput(t))
                        })
                        .on_submit(Message::CreateInstance(CreateInstanceMessage::SearchSubmit)),
                ]
                .push_maybe(
                    (!search_box.trim().is_empty())
                        .then_some(widget::text("Search Results:").size(12)),
                )
                .spacing(10);

                let sidebar = Self::get_sidebar_contents(
                    versions,
                    selected_version,
                    *is_server,
                    header.into(),
                    search_box,
                    selected_categories,
                );

                let view = row![
                    sidebar,
                    Self::get_main_page(
                        selected_version,
                        instance_name,
                        *download_assets,
                        existing_instances,
                        *is_server
                    )
                ]
                .width(Length::Fill);

                widget::stack!(view,)
                    .push_maybe(show_category_dropdown.then_some(widget::row![
                        widget::Space::with_width(97),
                        widget::column![
                            widget::Space::with_height(50),
                            ctxbox(Self::get_category_dropdown(selected_categories))
                        ]
                    ]))
                    .into()
            }
            MenuCreateInstance::DownloadingInstance(progress) => column![
                widget::text("Downloading Instance..").size(20),
                progress.view()
            ]
            .padding(10)
            .spacing(5)
            .into(),
            MenuCreateInstance::ImportingInstance(progress) => column![
                widget::text("Importing Instance..").size(20),
                progress.view()
            ]
            .padding(10)
            .spacing(5)
            .into(),
        }
    }

    fn get_main_page(
        selected_version: &ListEntry,
        instance_name: &String,
        download_assets: bool,
        existing_instances: Option<&[String]>,
        is_server: bool,
    ) -> widget::Column<'static, Message, LauncherTheme> {
        let already_exists = existing_instances.is_some_and(|n| n.contains(instance_name));

        column![
            widget::text!("Create {}", if is_server { "Server" } else { "Instance" })
                .size(24),
            row![
                widget::text("Name:").size(18),
                {
                    let placeholder = selected_version.name.as_str();
                    widget::text_input(placeholder, instance_name)
                        .on_input(|n| Message::CreateInstance(CreateInstanceMessage::NameInput(n)))
                }
            ].spacing(10).align_y(Alignment::Center),

            tooltip(
                row![
                    widget::Space::with_width(5),
                    widget::checkbox("Download assets?", download_assets).text_size(14).size(14).on_toggle(|t| Message::CreateInstance(CreateInstanceMessage::ChangeAssetToggle(t)))
                ],
                widget::text("If disabled, creating instance will be MUCH faster\nbut no sound or music will play").size(12),
                Position::FollowCursor
            ),
            widget::horizontal_rule(1),
            column![
                widget::text("- To install Fabric/Forge/OptiFine/etc and mods, click on Mods after installing the instance").size(12).style(tsubtitle),
                row!(
                    widget::text("- To sideload your own custom JARs, create an instance with a similar version, then go to").size(12).style(tsubtitle),
                    widget::text(" \"Edit->Custom Jar File\"").size(12).style(tsubtitle)
                ).wrap(),
            ].spacing(5),
            widget::vertical_space(),
            row![
                widget::horizontal_space(),
                tooltip(
                    widget::button(icon_manager::zip_file()).padding(iced::Padding::new(8.0).left(12.0).right(12.0))
                    .on_press(Message::CreateInstance(CreateInstanceMessage::Import)),
                    widget::text("Import Instance... (VERY EXPERIMENTAL right now)").size(14),
                    Position::Top
                ),
                get_create_button(already_exists),
            ].spacing(5)
        ].push_maybe({
            let real_platform = if cfg!(target_arch = "x86") { "x86_64" } else { "aarch64" };
            (cfg!(target_os = "linux") && (cfg!(target_arch = "x86") || cfg!(target_arch = "arm")))
            .then_some(column![
                // WARN: Linux i686 and arm32
                widget::text("Warning: On your platform (Linux 32 bit) only Minecraft 1.16.5 and below are supported.").size(20),
                widget::text!("If your computer isn't outdated, you might have wanted to download QuantumLauncher 64 bit ({real_platform})"),
            ])
        }).spacing(10).padding(10)
    }

    fn get_sidebar_contents<'a>(
        versions: Option<&'a [ListEntry]>,
        selected_version: &'a ListEntry,
        is_server: bool,
        header: Element<'static>,
        searchbox: &str,
        selected_categories: &HashSet<ListEntryKind>,
    ) -> widget::Container<'a, Message, LauncherTheme> {
        sidebar(
            "MenuCreateInstance:sidebar",
            Some(header),
            versions
                .into_iter()
                .flatten()
                .filter(|n| n.supports_server || !is_server)
                .filter(|n| selected_categories.contains(&n.kind))
                .filter(|n| {
                    searchbox.trim().is_empty()
                        || n.name
                            .to_lowercase()
                            .contains(&searchbox.trim().to_lowercase())
                })
                .map(|n| {
                    let label = widget::text(&n.name)
                        .size(if n.kind == ListEntryKind::Snapshot {
                            14
                        } else {
                            15
                        })
                        .style(|t: &LauncherTheme| {
                            t.style_text(if n.kind == ListEntryKind::Snapshot {
                                Color::SecondLight
                            } else {
                                Color::Light
                            })
                        });

                    sidebar_button(
                        n,
                        selected_version,
                        label,
                        Message::CreateInstance(CreateInstanceMessage::VersionSelected(n.clone())),
                    )
                }),
        )
    }

    fn get_category_dropdown(
        selected_categories: &HashSet<ListEntryKind>,
    ) -> widget::Column<'static, Message, LauncherTheme> {
        let mut col = column![widget::text("Version Types:").size(14)].spacing(5);

        for kind in ListEntryKind::ALL {
            let is_checked = selected_categories.contains(kind);
            col = col.push(
                widget::checkbox(kind.to_string(), is_checked)
                    .text_size(13)
                    .size(13)
                    .on_toggle(move |_| {
                        Message::CreateInstance(CreateInstanceMessage::CategoryToggle(*kind))
                    }),
            );
        }

        col
    }
}

fn get_create_button(already_exists: bool) -> Element<'static> {
    let create_button = button_with_icon(icon_manager::create(), "Create", 16).on_press_maybe(
        (!already_exists).then_some(Message::CreateInstance(CreateInstanceMessage::Start)),
    );

    if already_exists {
        tooltip(
            create_button,
            "An instance with that name already exists!",
            Position::FollowCursor,
        )
        .into()
    } else {
        create_button.into()
    }
}
