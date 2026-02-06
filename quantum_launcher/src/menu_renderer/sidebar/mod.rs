use iced::{
    widget::{self, column, row},
    Alignment, Length,
};
use ql_core::InstanceSelection;

use crate::{
    config::sidebar::{SidebarFolder, SidebarNode, SidebarNodeKind, SidebarSelection},
    menu_renderer::{
        ctx_button, ctxbox, offset, sidebar::drop_recv::drag_drop_receiver, underline,
        underline_maybe, Element, FONT_MONO,
    },
    state::{LaunchModal, Launcher, MainMenuMessage, MenuLaunch, Message, State},
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
        let is_drag_happening = matches!(menu.modal, Some(LaunchModal::Dragging { .. }));

        let button: Element = match &node.kind {
            SidebarNodeKind::Instance(_) => {
                self.get_node_instance(node, &selection, mode, is_selected)
            }
            SidebarNodeKind::Folder(f) => self.get_node_folder(node, &selection, mode, f),
        };

        widget::stack!(
            self.node_wrap_in_context_menu(selection.clone(), button),
            indent_guide_lines(mode, is_selected),
        )
        .push_maybe(
            (!is_drag_happening)
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
        let is_drag_happening = matches!(&menu.modal, Some(LaunchModal::Dragging { .. }));

        let drop_receiver = drag_drop_receiver(menu, selection, node);

        let text = widget::text(&node.name)
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
                column![node_button(
                    row![space, view.push_maybe(drop_receiver)],
                    is_drag_happening
                )
                .on_press(MainMenuMessage::ToggleFolderVisibility(folder.id).into())]
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
        let is_drag = matches!(&menu.modal, Some(LaunchModal::Dragging { .. }));

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
    ) -> widget::MouseArea<'a, Message, LauncherTheme> {
        widget::mouse_area(elem).on_right_press(
            MainMenuMessage::Modal(Some(LaunchModal::SidebarCtxMenu(
                Some(selection),
                self.window_state.mouse_pos,
            )))
            .into(),
        )
    }

    pub(super) fn sidebar_drag_tooltip<'a>(&'a self, menu: &'a MenuLaunch) -> Option<Element<'a>> {
        if let Some(LaunchModal::Dragging { being_dragged, .. }) = &menu.modal {
            if let Some(node) = self
                .config
                .sidebar
                .as_ref()
                .and_then(|n| n.get_node(being_dragged))
            {
                let node = self.get_node_rendered(menu, node, NodeMode::Dragged);
                let (x, y) = self.window_state.mouse_pos;
                let (winw, winh) = self.window_state.size;
                Some(offset(
                    node,
                    (x - 200.0).clamp(0.0, winw),
                    (y - 16.0).clamp(0.0, winh),
                ))
            } else {
                None
            }
        } else {
            None
        }
    }
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

pub fn context_menu(menu: &MenuLaunch) -> Option<Element<'_>> {
    if let Some(LaunchModal::SidebarCtxMenu(instance, (x, y))) = &menu.modal {
        let instance = instance.as_ref();
        Some(offset(
            // Could do something with instance-specific actions in the future
            ctxbox(
                column![ctx_button("New Folder")
                    .on_press(MainMenuMessage::NewFolder(instance.cloned()).into())]
                .push_maybe(instance.map(|_| widget::horizontal_rule(2)))
                .push_maybe(instance.and_then(|inst| {
                    if let SidebarSelection::Folder(id) = inst {
                        Some(
                            ctx_button("Delete Folder")
                                .on_press_with(|| MainMenuMessage::DeleteFolder(*id).into()),
                        )
                    } else {
                        None
                    }
                }))
                .spacing(4),
            )
            .width(150),
            *x,
            *y,
        ))
    } else {
        None
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
        MainMenuMessage::Modal(Some(LaunchModal::Dragging {
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
