use iced::{
    widget::{self, column, row},
    Alignment, Length,
};
use ql_core::InstanceSelection;

use crate::{
    config::sidebar::{FolderId, SidebarFolder, SidebarNode, SidebarNodeKind, SidebarSelection},
    icons,
    menu_renderer::{
        ctx_button, ctxbox, offset, sidebar::drop_recv::drag_drop_receiver, underline,
        underline_maybe, Element, CTXI_SIZE, FONT_MONO,
    },
    state::{
        EditInstanceMessage, LaunchModal, LaunchTab, Launcher, MainMenuMessage, MenuLaunch,
        Message, SidebarMessage, State,
    },
    stylesheet::{color::Color, styles::LauncherTheme, widgets::StyleButton},
};

mod drop_recv;

const LEVEL_WIDTH: u16 = 15;

#[derive(Clone, Copy)]
pub enum NodeMode {
    InTree(u16),
    Dragged,
}

impl NodeMode {
    pub fn get_space(self) -> widget::Space {
        widget::Space::with_width(match self {
            NodeMode::InTree(n) => LEVEL_WIDTH * n,
            NodeMode::Dragged => 0,
        })
    }
}

impl Launcher {
    pub(super) fn get_node_rendered<'a>(
        &'a self,
        menu: &'a MenuLaunch,
        node: &'a SidebarNode,
        mode: NodeMode,
    ) -> Element<'a> {
        // Tbh should be careful about careless heap allocations
        let selection = SidebarSelection::from_node(node);
        let is_selected = self.node_is_instance_selected(node);

        let show_drag_handle = !matches!(
            menu.modal,
            Some(LaunchModal::SDragging { .. } | LaunchModal::SRenamingFolder(_, _, _))
        );

        let button: Element = match &node.kind {
            SidebarNodeKind::Instance(_) => {
                self.get_node_instance(node, &selection, mode, is_selected)
            }
            SidebarNodeKind::Folder(f) => self.get_node_folder(node, &selection, mode, f),
        };

        widget::stack!(
            self.node_wrap_in_context_menu(selection.clone(), button, node.name.clone()),
            indent_guide_lines(mode, is_selected),
        )
        .push_maybe(
            show_drag_handle
                .then(|| widget::row![widget::horizontal_space(), drag_handle(&selection)]),
        )
        .into()
    }

    fn get_node_folder<'a>(
        &'a self,
        node: &'a SidebarNode,
        selection: &SidebarSelection,
        mode: NodeMode,
        folder: &'a SidebarFolder,
    ) -> Element<'a> {
        let State::Launch(menu) = &self.state else {
            return widget::Column::new().into();
        };
        let is_drag_happening = matches!(&menu.modal, Some(LaunchModal::SDragging { .. }));

        let drop_receiver = drag_drop_receiver(menu, selection, node);

        let text = if folder.is_expanded {
            widget::text(&node.name)
        } else {
            widget::text!("{}...", node.name)
        }
        .size(15)
        .style(move |t: &LauncherTheme| t.style_text(Color::Mid));

        let view = widget::stack!(underline(
            widget::row![
                widget::Space::with_width(2),
                widget::text(if folder.is_expanded { "- " } else { "+ " })
                    .font(FONT_MONO)
                    .size(14)
                    .style(move |t: &LauncherTheme| t.style_text(Color::Light)),
                text,
            ]
            .width(Length::Fill)
            .align_y(Alignment::Center)
            .padding([4, 10]),
            Color::Dark,
        ));

        let space = mode.get_space();

        match mode {
            NodeMode::InTree(nesting) => {
                let regular = || {
                    column![node_button(
                        row![space, view.push_maybe(drop_receiver)],
                        is_drag_happening
                    )
                    .on_press(SidebarMessage::ToggleFolderVisibility(folder.id).into())]
                };

                if let Some(LaunchModal::SRenamingFolder(id, name, is_creating)) = &menu.modal {
                    if folder.id == *id {
                        column![renaming_folder(*id, name, *is_creating)]
                    } else {
                        regular()
                    }
                } else {
                    regular()
                }
                .push_maybe(folder.is_expanded.then(|| {
                    widget::column(folder.children.iter().map(|node| {
                        self.get_node_rendered(menu, node, NodeMode::InTree(nesting + 1))
                    }))
                }))
                .into()
            }
            NodeMode::Dragged => drag_tooltip(row![space, view]).into(),
        }
    }

    fn get_node_instance<'a>(
        &'a self,
        node: &'a SidebarNode,
        selection: &SidebarSelection,
        mode: NodeMode,
        is_selected: bool,
    ) -> Element<'a> {
        let State::Launch(menu) = &self.state else {
            return widget::Column::new().into();
        };
        let is_drag = matches!(&menu.modal, Some(LaunchModal::SDragging { .. }));

        let text = widget::text(&node.name)
            .size(15)
            .style(move |t: &LauncherTheme| t.style_text(Color::SecondLight));

        let view = widget::stack!(underline_maybe(
            widget::row![widget::Space::with_width(2), text]
                .push_maybe(self.get_running_icon(menu, &node.name))
                .padding([5, 10])
                .width(Length::Fill),
            Color::Dark,
            !is_selected
        ));
        match mode {
            NodeMode::InTree(_) => node_button(
                row![
                    mode.get_space(),
                    view.push_maybe(drag_drop_receiver(menu, selection, node))
                ],
                is_drag,
            )
            .on_press_maybe((!is_selected).then(|| {
                MainMenuMessage::InstanceSelected(InstanceSelection::new(
                    &node.name,
                    menu.is_viewing_server,
                ))
                .into()
            }))
            .into(),
            NodeMode::Dragged => drag_tooltip(row![mode.get_space(), view]).into(),
        }
    }

    fn node_is_instance_selected(&self, node: &SidebarNode) -> bool {
        self.selected_instance
            .as_ref()
            .is_some_and(|sel| node == sel)
    }

    fn node_wrap_in_context_menu<'a>(
        &self,
        selection: SidebarSelection,
        elem: impl Into<Element<'a>>,
        name: String,
    ) -> widget::MouseArea<'a, Message, LauncherTheme> {
        widget::mouse_area(elem).on_right_press(
            MainMenuMessage::Modal(Some(LaunchModal::SCtxMenu(
                Some((selection, name)),
                self.window_state.mouse_pos,
            )))
            .into(),
        )
    }

    pub(super) fn sidebar_drag_tooltip<'a>(&'a self, menu: &'a MenuLaunch) -> Option<Element<'a>> {
        if let Some(LaunchModal::SDragging { being_dragged, .. }) = &menu.modal {
            if let Some(node) = self
                .config
                .sidebar
                .as_ref()
                .and_then(|n| n.get_node(being_dragged))
            {
                let node = self.get_node_rendered(menu, node, NodeMode::Dragged);
                let (x, y) = self.window_state.mouse_pos;
                let (winwidth, winheight) = self.window_state.size;
                Some(offset(
                    node,
                    (x - 200.0).clamp(0.0, winwidth),
                    (y - 16.0).clamp(0.0, winheight),
                ))
            } else {
                None
            }
        } else {
            None
        }
    }

    pub(super) fn sidebar_context_menu<'a>(menu: &'a MenuLaunch) -> Option<Element<'a>> {
        let Some(LaunchModal::SCtxMenu(instance, (x, y))) = &menu.modal else {
            return None;
        };

        let instance = instance.as_ref();

        let new_folder_b = ctx_button(icons::new_s(CTXI_SIZE), "New Folder")
            .on_press_with(move || SidebarMessage::NewFolder(instance.map(|n| n.0.clone())).into());

        let Some((inst, name)) = instance else {
            // Right clicked in empty space
            return Some(offset(ctxbox(new_folder_b).width(120), *x, *y));
        };

        Some(offset(
            ctxbox(
                column![
                    new_folder_b,
                    widget::Space::with_height(5),
                    widget::horizontal_rule(2),
                    widget::Space::with_height(5),
                    ctx_button(icons::file_s(CTXI_SIZE), "Change Icon"),
                    ctx_button(icons::edit_s(CTXI_SIZE), "Rename").on_press_with(
                        move || match inst {
                            SidebarSelection::Instance(name, kind) => {
                                Message::Multiple(vec![
                                    MainMenuMessage::InstanceSelected(InstanceSelection::new(
                                        name,
                                        kind.is_server(),
                                    ))
                                    .into(),
                                    MainMenuMessage::ChangeTab(LaunchTab::Edit).into(),
                                    EditInstanceMessage::RenameToggle.into(),
                                ])
                            }
                            SidebarSelection::Folder(folder_id) => {
                                MainMenuMessage::Modal(Some(LaunchModal::SRenamingFolder(
                                    *folder_id,
                                    name.clone(),
                                    false,
                                )))
                                .into()
                            }
                        }
                    ),
                ]
                .push_maybe(if let SidebarSelection::Folder(id) = inst {
                    Some(
                        ctx_button(icons::bin_s(CTXI_SIZE), "Delete Folder")
                            .on_press_with(|| SidebarMessage::DeleteFolder(*id).into()),
                    )
                } else {
                    None
                }),
            )
            .width(150),
            *x,
            *y,
        ))
    }
}

fn renaming_folder(
    id: FolderId,
    name: &str,
    is_creating: bool,
) -> widget::Row<'_, Message, LauncherTheme> {
    let text_input = widget::text_input("Enter name...", name)
        .id("MenuLaunch:rename_folder")
        .on_input(move |s| {
            MainMenuMessage::Modal(Some(LaunchModal::SRenamingFolder(id, s, is_creating))).into()
        })
        .on_submit(SidebarMessage::FolderRenameConfirm.into())
        .size(13)
        .padding([3, 5]);

    let done_button = widget::button(icons::checkmark_s(12))
        .style(|t: &LauncherTheme, s| t.style_button(s, StyleButton::RoundDark))
        .padding([4, 8])
        .on_press(SidebarMessage::FolderRenameConfirm.into());

    row![text_input, done_button]
        .push_maybe((!is_creating).then(|| {
            widget::button(icons::close_s(12))
                .style(|t: &LauncherTheme, s| t.style_button(s, StyleButton::RoundDark))
                .padding([4, 8])
                .on_press(MainMenuMessage::Modal(None).into())
        }))
        .align_y(Alignment::Center)
        .padding(5)
        .spacing(2)
}

/// The `| | |` lines in indentation. Eg:
///
/// ```txt
/// SomeFolder/
/// |- Instance
/// |- Folder/
/// |  |- Instance
/// |  |- Instance
/// ```
fn indent_guide_lines(
    nesting: NodeMode,
    is_selected: bool,
) -> widget::Row<'static, Message, LauncherTheme> {
    match nesting {
        NodeMode::InTree(nesting) => widget::row((0..nesting).map(|_| {
            row![
                widget::Space::with_width(LEVEL_WIDTH - 2),
                widget::vertical_rule(1).style(move |t: &LauncherTheme| t.style_rule(
                    if is_selected {
                        Color::Mid
                    } else {
                        Color::SecondDark
                    },
                    1
                ))
            ]
            .into()
        })),
        NodeMode::Dragged => widget::Row::new(),
    }
}

fn drag_tooltip<'a>(
    node_view: impl Into<Element<'a>>,
) -> widget::Container<'a, Message, LauncherTheme> {
    widget::container(node_view)
        .style(|t: &LauncherTheme| {
            t.style_container_bg_semiround([true; 4], Some((Color::ExtraDark, 0.9)))
        })
        .width(200)
}

fn drag_handle(selection: &SidebarSelection) -> widget::MouseArea<'static, Message, LauncherTheme> {
    widget::mouse_area(
        widget::row![widget::text("=")
            .size(20)
            .style(|t: &LauncherTheme| t.style_text(Color::ExtraDark))]
        .padding([0, 4])
        .align_y(Alignment::Center),
    )
    .on_press(
        MainMenuMessage::Modal(Some(LaunchModal::SDragging {
            being_dragged: selection.clone(),
            dragged_to: None,
        }))
        .into(),
    )
}

fn node_button<'a>(
    inner: impl Into<Element<'a>>,
    is_drag: bool,
) -> widget::Button<'a, Message, LauncherTheme> {
    widget::button(inner)
        .style(move |n: &LauncherTheme, status| {
            n.style_button(
                status,
                if is_drag {
                    StyleButton::FlatExtraDarkDead
                } else {
                    StyleButton::FlatExtraDark
                },
            )
        })
        .padding(0)
        .width(Length::Fill)
}
