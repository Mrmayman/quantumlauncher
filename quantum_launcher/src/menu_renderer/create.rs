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
        Column, Element, back_to_launch_screen, button_with_icon, dots, launch::import_description,
        shortcut_ctrl, sidebar_button, tooltip, tsubtitle,
    },
    state::{CreateInstanceMessage, MenuCreateInstance, MenuCreateInstanceChoosing, Message},
    stylesheet::{color::Color, styles::LauncherTheme, widgets::StyleButton},
};

impl MenuCreateInstance {
    pub fn view(&self, existing_instances: Option<&[String]>, timer: usize) -> Element<'_> {
        match self {
            MenuCreateInstance::Choosing(menu) => menu.view(existing_instances, timer).into(),
            MenuCreateInstance::DownloadingInstance(progress, kind) => column![
                widget::text!(
                    "Downloading {}...",
                    match kind {
                        InstanceKind::Server => "Server",
                        InstanceKind::Client => "Instance",
                    }
                )
                .size(20),
                progress.view()
            ]
            .padding(10)
            .spacing(5)
            .into(),
            MenuCreateInstance::ImportingInstance(progress) => column![
                widget::text("Importing Instance...").size(20),
                progress.view()
            ]
            .padding(10)
            .spacing(5)
            .into(),
        }
    }
}

impl MenuCreateInstanceChoosing {
    fn view(
        &self,
        existing_instances: Option<&[String]>,
        timer: usize,
    ) -> widget::PaneGrid<'_, Message, LauncherTheme> {
        widget::pane_grid(&self.sidebar_grid_state, |_, is_sidebar, _| {
            if *is_sidebar {
                self.get_sidebar_contents(timer).into()
            } else {
                self.get_main_page(existing_instances).into()
            }
        })
        .on_resize(10, |t| CreateInstanceMessage::SidebarResize(t.ratio).into())
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
            column![
                column![header].padding(10),
                widget::scrollable(widget::column(versions_iter.map(|n| {
                    let label = widget::text(&n.name).size(14).style(|t: &LauncherTheme| {
                        t.style_text(if n.kind == ListEntryKind::Snapshot {
                            Color::Mid
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
                    .into()
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

        let back_button = button_with_icon(icons::back_s(12), "Back", 13)
            .padding(pb)
            .style(|t: &LauncherTheme, s| t.style_button(s, StyleButton::RoundDark))
            .on_press(back_to_launch_screen(None));

        let enabled_servers = EXPERIMENTAL_SERVERS.read().is_ok_and(|n| *n);

        column![
            back_button,
            widget::text_input("Search...", &self.search_box)
                .size(14)
                .on_input(|t| CreateInstanceMessage::SearchInput(t).into())
                .on_submit(CreateInstanceMessage::SearchSubmit.into()),
        ]
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

        let mmc_import = EXPERIMENTAL_MMC_IMPORT.read().unwrap();

        let menu = column![
            row![
                widget::text!("Create {}", match self.kind {
                    InstanceKind::Client => "Instance",
                    InstanceKind::Server => "Server",
                })
                .size(24).width(Length::Fill),
            ]
            .push_maybe(mmc_import.then_some(tooltip(
                widget::button(import_description())
                    .padding([4, 8])
                    .on_press(CreateInstanceMessage::Import.into()),
                widget::text("Import Instance... (VERY EXPERIMENTAL right now)").size(14),
                Position::Top
            ))),
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
        .push(
            widget::text("To sideload your own custom JARs, create an instance with a similar version, then go to \"Edit->Custom Jar File\"")
                .size(12)
                .style(|t: &LauncherTheme| t.style_text(Color::Mid)),
        )
        .push_maybe({
            let real_platform = if cfg!(target_arch = "x86") { "x86_64" } else { "aarch64" };
            cfg!(target_pointer_width = "32").then_some(column![
                // WARN: 32-bit
                widget::text("Minecraft 1.20.5 and above dropped support for 32-bit systems.").size(20),
                widget::text!("If your computer isn't outdated, you might have wanted to download QuantumLauncher 64 bit ({real_platform})"),
            ])
        })
        .push(widget::vertical_space())
        .push(row![
            widget::horizontal_space(),
            get_create_button(already_exists),
        ])
        .spacing(12).padding(16);

        widget::container(column![
            menu,
            widget::horizontal_rule(1)
                .style(|t: &LauncherTheme| t.style_rule(Color::SecondDark, 1)),
            Self::get_version_filters(&self.selected_categories)
        ])
        .padding(5)
        .style(|t: &LauncherTheme| t.style_container_bg(0.0, None))
        .into()
    }

    fn get_version_filters(selected_categories: &HashSet<ListEntryKind>) -> Column<'static> {
        let list = widget::row(ListEntryKind::ALL.iter().map(|kind| {
            let is_checked = selected_categories.contains(kind);
            let mut label = kind.to_string();
            label.push(' ');
            widget::checkbox(label, is_checked)
                .text_size(12)
                .size(12)
                .spacing(4)
                .style(|t: &LauncherTheme, s| t.style_checkbox(s, Some(Color::SecondLight)))
                .on_toggle(move |_| CreateInstanceMessage::CategoryToggle(*kind).into())
                .into()
        }))
        .spacing(6)
        .wrap();

        let some_versions_are_hidden = (selected_categories.len() != ListEntryKind::ALL.len())
            .then_some(
                widget::text!(
                    "Some versions are hidden {}",
                    if selected_categories.contains(&ListEntryKind::Release) {
                        ""
                    } else {
                        "(!)"
                    }
                )
                .size(10)
                .style(tsubtitle),
            );

        column![
            row![widget::text("Version Types: ").width(Length::Fill),]
                .push_maybe(some_versions_are_hidden),
            list
        ]
        .spacing(5)
        .padding(10)
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
