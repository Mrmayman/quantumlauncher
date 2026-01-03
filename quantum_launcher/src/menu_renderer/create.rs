use std::collections::HashSet;

use iced::{
    widget::{self, column, row, tooltip::Position},
    Alignment, Length,
};
use ql_core::ListEntryKind;

use crate::{
    cli::EXPERIMENTAL_MMC_IMPORT,
    icons,
    menu_renderer::{
        button_with_icon, ctxbox, dots, shortcut_ctrl, sidebar_button, tooltip, tsubtitle, Element,
    },
    state::{CreateInstanceMessage, MenuCreateInstance, MenuCreateInstanceChoosing, Message},
    stylesheet::{
        color::Color,
        styles::{LauncherTheme, BORDER_RADIUS, BORDER_WIDTH},
        widgets::StyleButton,
    },
};

impl MenuCreateInstance {
    pub fn view<'a>(&'a self, existing_instances: Option<&[String]>, timer: usize) -> Element<'a> {
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
    pub fn view(&self, existing_instances: Option<&[String]>, timer: usize) -> Element<'_> {
        let view = widget::pane_grid(&self.sidebar_grid_state, |_, is_sidebar, _| {
            if *is_sidebar {
                self.get_sidebar_contents(timer).into()
            } else {
                self.get_main_page(existing_instances).into()
            }
        })
        .on_resize(10, |t| {
            Message::CreateInstance(CreateInstanceMessage::SidebarResize(t.ratio))
        });

        widget::stack!(
            view,
            self.show_category_dropdown.then_some(row![
                widget::space().width(90),
                column![
                    widget::space().height(40),
                    ctxbox(Self::get_category_dropdown(&self.selected_categories))
                ]
            ])
        )
        .into()
    }

    fn get_sidebar_contents<'a>(
        &'a self,
        timer: usize,
    ) -> widget::Container<'a, Message, LauncherTheme> {
        let header = self.get_sidebar_header();

        let Some(versions) = &self.list else {
            return widget::container(
                column![
                    header,
                    widget::text!("Loading versions{}", dots(timer))
                        .style(tsubtitle)
                        .size(12)
                ]
                .spacing(10)
                .padding(10),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|t: &LauncherTheme| t.style_container_sharp_box(0.0, Color::ExtraDark));
        };

        let versions_iter = versions
            .iter()
            .filter(|n| n.supports_server || !self.is_server)
            .filter(|n| self.selected_categories.contains(&n.kind))
            .filter(|n| {
                self.search_box.trim().is_empty()
                    || n.name
                        .to_lowercase()
                        .contains(&self.search_box.trim().to_lowercase())
            });

        widget::container(
            column![
                column![header].padding(10),
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
                        Message::CreateInstance(CreateInstanceMessage::VersionSelected(n.clone())),
                    )
                }),))
                .style(LauncherTheme::style_scrollable_flat_extra_dark)
                .height(Length::Fill)
                .id(widget::Id::new("MenuCreateInstance:sidebar"))
            ]
            .spacing(10),
        )
        .width(Length::Fill)
        .style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::ExtraDark))
    }

    fn get_sidebar_header(&self) -> widget::Column<'_, Message, LauncherTheme> {
        let opened_controls = self.show_category_dropdown;
        let hidden = self.selected_categories.len() == ListEntryKind::ALL.len();

        column![
            row![
                self.gback_button(),
                gfilters_button(opened_controls, hidden)
            ]
            .spacing(5)
            .wrap(),
            self.ghidden_versions(hidden),
            widget::text_input("Search...", &self.search_box)
                .size(14)
                .on_input(|t| Message::CreateInstance(CreateInstanceMessage::SearchInput(t)))
                .on_submit(Message::CreateInstance(CreateInstanceMessage::SearchSubmit)),
            (!self.search_box.trim().is_empty())
                .then_some(widget::text("Search Results:").style(tsubtitle).size(12))
        ]
        .spacing(7)
    }

    fn ghidden_versions(&self, hidden: bool) -> Option<widget::Text<LauncherTheme>> {
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
        )
    }

    fn gback_button(&self) -> widget::Button<'_, Message, LauncherTheme> {
        button_with_icon(icons::back_s(12), "Back", 13)
            .padding([4, 10])
            .style(|t: &LauncherTheme, s| t.style_button(s, StyleButton::RoundDark))
            .on_press(Message::LaunchScreenOpen {
                message: None,
                clear_selection: false,
                is_server: Some(self.is_server),
            })
    }

    fn get_main_page(&self, existing_instances: Option<&[String]>) -> Element<'_> {
        let already_exists = existing_instances.is_some_and(|n| {
            n.contains(&self.instance_name)
                || (self.instance_name.is_empty() && n.contains(&self.selected_version.name))
        });

        let main_part = column![
            widget::text!("Create {}", if self.is_server { "Server" } else { "Instance" })
                .size(24),
            row![
                widget::text("Name:").size(18),
                {
                    let placeholder = self.selected_version.name.as_str();
                    widget::text_input(placeholder, &self.instance_name)
                        .on_input(|n| Message::CreateInstance(CreateInstanceMessage::NameInput(n)))
                }
            ].spacing(10).align_y(Alignment::Center),

            tooltip(
                row![
                    widget::space().width(5),
                    widget::checkbox(self.download_assets).size(14).on_toggle(|t| Message::CreateInstance(CreateInstanceMessage::ChangeAssetToggle(t))),
                    widget::text("Download assets?").size(14),
                ].spacing(5),
                widget::text("If disabled, creating instance will be MUCH faster\nbut no sound or music will play").size(12),
                Position::FollowCursor
            ),
            widget::rule::horizontal(1),
            widget::text("- To install Fabric/Forge/OptiFine/etc and mods, click on Mods after installing the instance").size(12).style(tsubtitle),
            row!(
                widget::text("- To sideload your own custom JARs, create an instance with a similar version, then go to \"Edit->Custom Jar File\"").size(12).style(tsubtitle)
            ).wrap(),
            (cfg!(target_os = "linux") && (cfg!(target_arch = "x86") || cfg!(target_arch = "arm")))
            .then_some(column![
                // WARN: Linux i686 and arm32
                widget::text("Warning: On your platform (Linux 32 bit) only Minecraft 1.16.5 and below are supported.").size(20),
                widget::text!("If your computer isn't outdated, you might have wanted to download QuantumLauncher 64 bit ({})",
                    if cfg!(target_arch = "x86") { "x86_64" } else { "aarch64" }),
            ])
        ].spacing(12);

        let menu = column![
            column![main_part, widget::space().height(Length::Fill)],
            row![
                widget::space().width(Length::Fill),
                EXPERIMENTAL_MMC_IMPORT.read().unwrap().then_some(tooltip(
                    button_with_icon(icons::upload(), "Import from MultiMC...", 16)
                        .on_press(Message::CreateInstance(CreateInstanceMessage::Import)),
                    widget::text("Import Instance... (VERY EXPERIMENTAL right now)").size(14),
                    Position::Top
                )),
                get_create_button(already_exists)
            ]
            .spacing(5)
        ]
        .spacing(10)
        .padding(16);

        widget::container(widget::container(menu).style(|t: &LauncherTheme| {
            t.style_container_round_box(BORDER_WIDTH, Color::Dark, BORDER_RADIUS)
        }))
        .padding(5)
        .style(|t: &LauncherTheme| t.style_container_sharp_box(0.0, Color::ExtraDark))
        .into()
    }

    fn get_category_dropdown(
        selected_categories: &HashSet<ListEntryKind>,
    ) -> widget::Column<'static, Message, LauncherTheme> {
        column![widget::text("Version Types:").size(14)]
            .extend(ListEntryKind::ALL.iter().map(|kind| {
                let is_checked = selected_categories.contains(kind);

                widget::row![
                    widget::checkbox(is_checked).size(13).on_toggle(move |_| {
                        Message::CreateInstance(CreateInstanceMessage::CategoryToggle(*kind))
                    }),
                    widget::text(kind.to_string()).size(13)
                ]
                .spacing(5)
                .into()
            }))
            .spacing(5)
    }
}

fn gfilters_button(
    opened_controls: bool,
    hidden: bool,
) -> widget::Button<'static, Message, LauncherTheme> {
    button_with_icon(
        icons::filter_s(12),
        if hidden { "Filters" } else { "Filters •" },
        13,
    )
    .padding([4, 10])
    .style(move |t: &LauncherTheme, s| {
        t.style_button(
            s,
            if opened_controls {
                StyleButton::Round
            } else {
                StyleButton::RoundDark
            },
        )
    })
    .on_press(Message::CreateInstance(
        CreateInstanceMessage::ContextMenuToggle,
    ))
}

fn get_create_button(already_exists: bool) -> widget::Tooltip<'static, Message, LauncherTheme> {
    let create_button = button_with_icon(icons::new(), "Create", 16).on_press_maybe(
        (!already_exists).then_some(Message::CreateInstance(CreateInstanceMessage::Start)),
    );

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
