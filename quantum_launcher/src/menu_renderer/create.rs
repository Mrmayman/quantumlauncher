use std::collections::HashSet;

use iced::{
    Alignment, Length,
    widget::{self, column, row, tooltip::Position},
};
use ql_core::{InstanceKind, ListEntryKind};

use crate::{
    cli::{EXPERIMENTAL_MMC_IMPORT, EXPERIMENTAL_SERVERS},
    icons,
    menu_renderer::{
        Column, Element, back_to_launch_screen, button_with_icon, ctxbox, dots,
        launch::import_description, offset, shortcut_ctrl, sidebar_button, tooltip, tsubtitle,
    },
    state::{CreateInstanceMessage, MenuCreateInstance, MenuCreateInstanceChoosing, Message},
    stylesheet::{
        color::Color,
        styles::{BORDER_RADIUS, BORDER_WIDTH, LauncherTheme},
        widgets::StyleButton,
    },
};

impl MenuCreateInstance {
    pub fn view(&self, existing_instances: Option<&[String]>, timer: usize) -> Element<'_> {
        match self {
            MenuCreateInstance::Choosing(menu) => menu.view(existing_instances, timer),
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
}

impl MenuCreateInstanceChoosing {
    fn view(&self, existing_instances: Option<&[String]>, timer: usize) -> Element<'_> {
        let view = widget::pane_grid(&self.sidebar_grid_state, |_, is_sidebar, _| {
            if *is_sidebar {
                self.get_sidebar_contents(timer).into()
            } else {
                self.get_main_page(existing_instances).into()
            }
        })
        .on_resize(10, |t| CreateInstanceMessage::SidebarResize(t.ratio).into());

        widget::stack!(view)
            .push_maybe(self.show_category_dropdown.then_some(offset(
                ctxbox(Self::get_category_dropdown(&self.selected_categories)),
                90,
                40,
            )))
            .into()
    }

    fn get_sidebar_contents(&self, timer: usize) -> widget::Container<'_, Message, LauncherTheme> {
        fn side_box<'a>(
            e: impl Into<Element<'a>>,
        ) -> widget::Container<'a, Message, LauncherTheme> {
            widget::container(e)
                .width(Length::Fill)
                .height(Length::Fill)
                .style(|t: &LauncherTheme| t.style_container_sharp_box(0.0, Color::ExtraDark))
        }

        let header = self.get_sidebar_header();

        let versions = match &self.list {
            Ok(Some(v)) => v,
            Ok(None) => {
                return side_box(
                    column![
                        header,
                        widget::text!("Loading versions{}", dots(timer))
                            .style(tsubtitle)
                            .size(12)
                    ]
                    .spacing(10)
                    .padding(10),
                );
            }
            Err(err) => {
                return side_box(
                    column![
                        header,
                        widget::text!("Failed to load versions:\n\n{err}")
                            .style(tsubtitle)
                            .size(12)
                    ]
                    .spacing(10)
                    .padding(10),
                );
            }
        };

        let versions_iter = versions
            .iter()
            .filter(|n| n.supports_server || !matches!(self.kind, InstanceKind::Server))
            .filter(|n| self.selected_categories.contains(&n.kind))
            .filter(|n| {
                self.search_box.trim().is_empty()
                    || n.name
                        .to_lowercase()
                        .contains(&self.search_box.trim().to_lowercase())
            });

        side_box(
            widget::column![
                widget::column![header].padding(10),
                widget::scrollable(widget::column(versions_iter.map(|n| {
                    let label = widget::text(&n.name).size(14).style(|t: &LauncherTheme| {
                        t.style_text(if n.kind == ListEntryKind::Snapshot {
                            Color::SecondLight
                        } else {
                            Color::Light
                        })
                    });

                    sidebar_button(
                        n,
                        &self.selected_version,
                        label,
                        CreateInstanceMessage::VersionSelected(n.clone()).into(),
                    )
                })))
                .spacing(0)
                .style(LauncherTheme::style_scrollable_flat_extra_dark)
                .height(Length::Fill)
                .id(widget::scrollable::Id::new("MenuCreateInstance:sidebar"))
            ]
            .padding(iced::Padding::new(0.0).right(5.0)),
        )
    }

    fn get_sidebar_header(&self) -> Column<'_> {
        let pb = [4, 10];
        let opened_controls = self.show_category_dropdown;
        let hidden = self.selected_categories.len() == ListEntryKind::ALL.len();

        let buttons = row![
            button_with_icon(icons::back_s(12), "Back", 13)
                .padding(pb)
                .style(|t: &LauncherTheme, s| t.style_button(s, StyleButton::RoundDark))
                .on_press(back_to_launch_screen(None)),
            button_with_icon(
                icons::filter_s(12),
                if hidden { "Filters" } else { "Filters •" },
                13
            )
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
        .spacing(5)
        .wrap();

        let enabled_servers = EXPERIMENTAL_SERVERS.read().is_ok_and(|n| *n);

        column![buttons]
            .push_maybe(
                (!hidden).then_some(
                    widget::text!(
                        "Some versions are hidden {}\n(Click \"Filters\" to show)",
                        if self.selected_categories.contains(&ListEntryKind::Release) {
                            ""
                        } else {
                            "(!)"
                        }
                    )
                    .size(10)
                    .style(tsubtitle),
                ),
            )
            .push(
                widget::text_input("Search...", &self.search_box)
                    .size(14)
                    .on_input(|t| CreateInstanceMessage::SearchInput(t).into())
                    .on_submit(CreateInstanceMessage::SearchSubmit.into()),
            )
            .push_maybe(
                (!self.search_box.trim().is_empty())
                    .then_some(widget::text("Search Results:").style(tsubtitle).size(12)),
            )
            .push_maybe(enabled_servers.then(|| {
                let radio = |l, v| {
                    widget::radio(l, v, Some(self.kind), |t| {
                        CreateInstanceMessage::ChangeKind(t).into()
                    })
                    .spacing(4)
                    .size(12)
                    .text_size(12)
                };
                row![
                    widget::text("Create:").size(12),
                    radio("Instance", InstanceKind::Client),
                    radio("Server", InstanceKind::Server)
                ]
                .spacing(4)
                .align_y(Alignment::Center)
                .wrap()
            }))
            .spacing(7)
    }

    fn get_main_page(&self, existing_instances: Option<&[String]>) -> Element<'_> {
        let already_exists = existing_instances.is_some_and(|n| {
            n.contains(&self.instance_name)
                || (self.instance_name.is_empty() && n.contains(&self.selected_version.name))
        });

        let main_part = column![
            widget::text!("Create {}", match self.kind {
                InstanceKind::Client => "Instance",
                InstanceKind::Server => "Server",
            })
                .size(24),
            row![
                widget::text("Name:").size(18),
                match self.kind {
                    InstanceKind::Server => widget::text_input(&format!("{} server", self.selected_version.name), &self.instance_name),
                    InstanceKind::Client => widget::text_input(&self.selected_version.name, &self.instance_name),
                }
                .on_input(|n| CreateInstanceMessage::NameInput(n).into())

            ].spacing(10).align_y(Alignment::Center),
        ]

        .push_maybe(matches!(self.kind, InstanceKind::Client).then(|| tooltip(
            row![
                widget::Space::with_width(5),
                widget::checkbox("Download assets?", self.download_assets).text_size(14).size(14).on_toggle(|t| Message::CreateInstance(CreateInstanceMessage::ChangeAssetToggle(t)))
            ],
            widget::text("If disabled, creating instance will be MUCH faster\nbut no sound or music will play").size(12),
            Position::FollowCursor
        )))
        .push(widget::horizontal_rule(1))

        .push(
            widget::text("To sideload your own custom JARs, create an instance with a similar version, then go to \"Edit->Custom Jar File\"").size(12).style(tsubtitle),
        )
        .push_maybe({
            let real_platform = if cfg!(target_arch = "x86") { "x86_64" } else { "aarch64" };
            cfg!(target_pointer_width = "32").then_some(column![
                // WARN: 32-bit
                widget::text("Minecraft 1.20.5 and above dropped support for 32-bit systems.").size(20),
                widget::text!("If your computer isn't outdated, you might have wanted to download QuantumLauncher 64 bit ({real_platform})"),
            ])
        }).spacing(12);

        let mmc_import = EXPERIMENTAL_MMC_IMPORT.read().unwrap();

        let menu = column![
            main_part,
            widget::vertical_space(),
            widget::Row::new()
                .push_maybe(
                    mmc_import.then_some(tooltip(
                        widget::button(import_description())
                            .padding([4, 8])
                            .on_press(CreateInstanceMessage::Import.into()),
                        widget::text("Import Instance... (VERY EXPERIMENTAL right now)").size(14),
                        Position::Top
                    ))
                )
                .push(widget::horizontal_space())
                .push(get_create_button(already_exists))
                .align_y(Alignment::End)
                .spacing(5)
        ]
        .spacing(10)
        .padding(16);

        widget::container(widget::container(menu).style(|t: &LauncherTheme| {
            widget::container::Style {
                border: {
                    iced::Border {
                        color: t.get(Color::SecondDark),
                        width: BORDER_WIDTH,
                        radius: BORDER_RADIUS.into(),
                    }
                },
                background: Some(t.get_bg(Color::Dark)),
                ..Default::default()
            }
        }))
        .padding(5)
        .style(|t: &LauncherTheme| t.style_container_sharp_box(0.0, Color::ExtraDark))
        .into()
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
                    .on_toggle(move |_| CreateInstanceMessage::CategoryToggle(*kind).into()),
            );
        }

        col
    }
}

fn get_create_button(already_exists: bool) -> widget::Tooltip<'static, Message, LauncherTheme> {
    let create_button = button_with_icon(icons::new(), "Create", 16)
        .on_press_maybe((!already_exists).then_some(CreateInstanceMessage::Start.into()));

    if already_exists {
        tooltip(
            create_button,
            "An instance with that name already exists!",
            Position::FollowCursor,
        )
    } else {
        tooltip(create_button, shortcut_ctrl("Enter"), Position::Bottom)
    }
}
