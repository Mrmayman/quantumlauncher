use std::sync::Arc;

use iced::{
    Alignment, Length,
    widget::{self, column, row},
};
use ql_core::{Instance, InstanceKind};

use crate::{
    config::sidebar::{FolderId, SidebarFolder, SidebarNode, SidebarNodeKind, SidebarSelection},
    icons,
    menu_renderer::{
        CTXI_SIZE, Element, FONT_MONO, ctx_button_icon, ctxbox, offset,
        sidebar::drop_recv::drag_drop_receiver, underline, underline_maybe,
    },
    state::{
        EditInstanceMessage, LaunchModal, LaunchTab, Launcher, MainMenuMessage, MenuLaunch,
        Message, SidebarMessage, State,
    },
    stylesheet::{
        color::Color,
        styles::{LauncherTheme, mix},
        widgets::StyleButton,
    },
};

mod drop_recv;

const LEVEL_WIDTH: u16 = 15;

#[derive(Clone, Copy)]
pub enum NodeMode {
    InTree(u16),
    Dragged,
}

impl NodeMode {
    fn get_space(self) -> widget::Space {
        widget::Space::with_width(match self {
            NodeMode::InTree(n) => LEVEL_WIDTH * n,
            NodeMode::Dragged => 0,
        })
    }

    fn get_button<'a>(
        self,
        inner: impl Into<Element<'a>>,
    ) -> widget::Button<'a, Message, LauncherTheme> {
        widget::button(row![self.get_space(), inner.into()])
            .style(|n: &LauncherTheme, status| n.style_button(status, StyleButton::FlatExtraDark))
            .padding(0)
            .width(Length::Fill)
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
            SidebarNodeKind::Instance(kind) => {
                self.get_node_instance(node, &selection, mode, is_selected, *kind)
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

        let view = Self::create_folder_view(node, folder);

        match mode {
            NodeMode::InTree(nesting) => {
                let view = view.push_maybe(drag_drop_receiver(menu, selection, node));

                let msg = SidebarMessage::ToggleFolderVisibility(folder.id).into();
                let inner: Element = if let Some(LaunchModal::SRenamingFolder(
                    id,
                    name,
                    is_creating,
                )) = &menu.modal
                {
                    if folder.id == *id {
                        renaming_folder(*id, name, *is_creating).into()
                    } else {
                        mode.get_button(view).on_press(msg).into()
                    }
                } else {
                    mode.get_button(view).on_press(msg).into()
                };

                column![inner]
                    .push_maybe(folder.is_expanded.then(|| {
                        widget::column(folder.children.iter().map(|node| {
                            self.get_node_rendered(menu, node, NodeMode::InTree(nesting + 1))
                        }))
                    }))
                    .into()
            }
            NodeMode::Dragged => drag_tooltip(row![mode.get_space(), view]).into(),
        }
    }

    fn get_node_instance<'a>(
        &'a self,
        node: &'a SidebarNode,
        selection: &SidebarSelection,
        mode: NodeMode,
        is_selected: bool,
        kind: InstanceKind,
    ) -> Element<'a> {
        let State::Launch(menu) = &self.state else {
            return widget::Column::new().into();
        };

        let text = widget::text(&*node.name)
            .size(15)
            .style(move |t: &LauncherTheme| t.style_text(Color::SecondLight));

        let view = widget::stack!(underline_maybe(
            widget::row![text]
                .push_maybe(self.get_running_icon(&node.name, kind))
                .padding([5, 14])
                .width(Length::Fill)
                .align_y(Alignment::Center),
            Color::Dark,
            !is_selected
        ));

        match mode {
            NodeMode::InTree(_) => mode
                .get_button(view.push_maybe(drag_drop_receiver(menu, selection, node)))
                .on_press_maybe((!is_selected).then(|| {
                    MainMenuMessage::InstanceSelected(Instance::new(&node.name, kind)).into()
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
        name: Arc<str>,
    ) -> widget::MouseArea<'a, Message, LauncherTheme> {
        widget::mouse_area(elem).on_right_press(
            MainMenuMessage::Modal(Some(LaunchModal::SCtxMenu(
                Some((selection, name)),
                self.window_state.mouse_pos,
            )))
            .into(),
        )
    }

    pub(super) fn sidebar_drag_tooltip<'a>(
        &'a self,
        menu: &'a MenuLaunch,
    ) -> Option<widget::Row<'a, Message, LauncherTheme>> {
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

    pub(super) fn sidebar_context_menu(
        menu: &MenuLaunch,
    ) -> Option<widget::Row<'_, Message, LauncherTheme>> {
        let Some(LaunchModal::SCtxMenu(instance, (x, y))) = &menu.modal else {
            return None;
        };

        let instance = instance.as_ref();

        let new_folder_b = ctx_button_icon(icons::new_s(CTXI_SIZE), "New Folder")
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
                    // ctx_button_icon(icons::file_s(CTXI_SIZE), "Change Icon"),
                    ctx_button_icon(icons::edit_s(CTXI_SIZE), "Rename").on_press_with(move || {
                        match inst {
                            SidebarSelection::Instance(name, kind) => Message::Multiple(vec![
                                MainMenuMessage::InstanceSelected(Instance::new(name, *kind))
                                    .into(),
                                MainMenuMessage::ChangeTab(LaunchTab::Edit).into(),
                                EditInstanceMessage::RenameToggle.into(),
                            ]),
                            SidebarSelection::Folder(folder_id) => MainMenuMessage::Modal(Some(
                                LaunchModal::SRenamingFolder(*folder_id, name.to_string(), false),
                            ))
                            .into(),
                        }
                    }),
                ]
                .push_maybe(if let SidebarSelection::Folder(id) = inst {
                    Some(
                        ctx_button_icon(icons::bin_s(CTXI_SIZE), "Delete Folder")
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

    fn create_folder_view<'a>(
        node: &'a SidebarNode,
        folder: &SidebarFolder,
    ) -> widget::Stack<'a, Message, LauncherTheme> {
        let text = if folder.is_expanded {
            widget::text(&*node.name)
        } else {
            widget::text!("{}...", node.name)
        }
        .size(15)
        .style(move |t: &LauncherTheme| t.style_text(Color::Mid));

        let bottom_bar =
            widget::horizontal_rule(0.5).style(|t: &LauncherTheme| widget::rule::Style {
                color: mix(t.get(Color::Dark), t.get(Color::SecondDark)),
                width: 1,
                radius: 0.into(),
                fill_mode: widget::rule::FillMode::Full,
            });

        let expand_sign = widget::text(if folder.is_expanded { "- " } else { "+ " })
            .font(FONT_MONO)
            .size(14)
            .style(move |t: &LauncherTheme| t.style_text(Color::Light));

        widget::stack!(
            underline(
                widget::row![widget::Space::with_width(2), expand_sign, text]
                    .width(Length::Fill)
                    .align_y(Alignment::Center)
                    .padding([5, 10]),
                Color::Dark,
            ),
            bottom_bar
        )
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

    row![text_input, done_button,]
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
        widget::row![
            widget::text(":")
                .size(16)
                .style(|t: &LauncherTheme| widget::text::Style {
                    // color: Some(mix(t.get(Color::Mid), t.get(Color::Dark))),
                    color: Some(mix(t.get(Color::Mid), t.get(Color::SecondDark))),
                    // color: Some(t.get(Color::Mid)),
                })
        ]
        .padding([3, 14]),
    )
    .on_press(
        MainMenuMessage::Modal(Some(LaunchModal::SDragging {
            being_dragged: selection.clone(),
            dragged_to: None,
        }))
        .into(),
    )
}
