use iced::{
    widget::{self, column, row},
    Alignment, Length,
};
use ql_core::InstanceSelection;

use crate::{
    config::sidebar::{SDragLocation, SidebarNode, SidebarNodeKind, SidebarSelection},
    menu_renderer::{underline, underline_maybe, Element},
    state::{LaunchModal, Launcher, MainMenuMessage, MenuLaunch, Message},
    stylesheet::{color::Color, styles::LauncherTheme, widgets::StyleButton},
};

impl Launcher {
    pub(super) fn get_node_rendered<'a>(
        &'a self,
        menu: &'a MenuLaunch,
        node: &'a SidebarNode,
        nesting: i16,
    ) -> Element<'a> {
        const DRAGGED_TOOLTIP: i16 = -1;
        const LEVEL_WIDTH: u16 = 15;

        // Tbh should be careful about careless heap allocations
        let selection = SidebarSelection::from_node(node);
        let is_selected = self.node_is_instance_selected(node);
        let is_drag = matches!(&menu.modal, Some(LaunchModal::Dragging { .. }));
        let is_being_dragged =
            if let Some(LaunchModal::Dragging { being_dragged, .. }) = &menu.modal {
                *being_dragged == selection && nesting != DRAGGED_TOOLTIP
            } else {
                false
            };

        let text = widget::text(&node.name)
            .size(15)
            .style(move |t: &LauncherTheme| {
                t.style_text(if is_being_dragged {
                    Color::Dark
                } else {
                    Color::SecondLight
                })
            });

        let nesting_inner = widget::Space::with_width(if nesting > 0 {
            LEVEL_WIDTH * nesting as u16
        } else {
            0
        });
        let nesting_outer = move |c| {
            widget::row((0..nesting).into_iter().map(|_| {
                row![
                    widget::Space::with_width(LEVEL_WIDTH - 2),
                    widget::vertical_rule(1).style(move |t: &LauncherTheme| t.style_rule(c, 1))
                ]
                .into()
            }))
        };

        let drop_receiver = drag_drop_receiver(menu, &selection, node, nesting);

        let button: Element = match &node.kind {
            SidebarNodeKind::Instance(_) => {
                let node_view = row![
                    nesting_inner,
                    widget::stack!(underline_maybe(
                        widget::row![widget::Space::with_width(2), text]
                            .push_maybe(self.get_running_icon(menu, &node.name))
                            .padding([5, 10])
                            .width(Length::Fill),
                        Color::Dark,
                        !is_selected
                    ))
                    .push_maybe(drop_receiver)
                ];
                if nesting == DRAGGED_TOOLTIP {
                    drag_tooltip(node_view).into()
                } else {
                    node_button(node_view, is_drag)
                        .on_press_maybe((!is_selected).then(|| {
                            MainMenuMessage::InstanceSelected(InstanceSelection::new(
                                &node.name,
                                menu.is_viewing_server,
                            ))
                            .into()
                        }))
                        .into()
                }
            }
            SidebarNodeKind::Folder {
                id,
                children,
                is_expanded,
            } => {
                let inner = row![
                    nesting_inner,
                    widget::stack!(underline_maybe(
                        widget::row![
                            widget::Space::with_width(2),
                            widget::text(if *is_expanded { "v  " } else { ">  " })
                                .size(14)
                                .style(move |t: &LauncherTheme| t.style_text(
                                    if is_being_dragged {
                                        Color::Mid
                                    } else {
                                        Color::Light
                                    }
                                )),
                            underline(text, Color::SecondDark),
                        ]
                        .width(Length::Fill)
                        .align_y(Alignment::Center)
                        .padding([4, 10]),
                        Color::Dark,
                        !is_selected
                    ))
                    .push_maybe(drop_receiver)
                ];
                if nesting == DRAGGED_TOOLTIP {
                    drag_tooltip(inner).into()
                } else {
                    column![node_button(inner, is_drag)
                        .on_press(MainMenuMessage::ToggleFolderVisibility(*id).into())]
                    .push_maybe(is_expanded.then(|| {
                        widget::column(
                            children
                                .iter()
                                .map(|node| self.get_node_rendered(menu, node, nesting + 1)),
                        )
                    }))
                    .into()
                }
            }
        };

        widget::stack!(
            self.node_wrap_in_context_menu(&selection, button),
            nesting_outer(if is_selected {
                Color::Mid
            } else {
                Color::SecondDark
            }),
        )
        .push_maybe(
            (!is_drag).then(|| widget::row![widget::horizontal_space(), drag_handle(&selection)]),
        )
        .into()
    }

    fn node_is_instance_selected(&self, node: &SidebarNode) -> bool {
        self.selected_instance
            .as_ref()
            .is_some_and(|sel| node == sel)
    }

    fn node_wrap_in_context_menu<'a>(
        &self,
        selection: &SidebarSelection,
        elem: impl Into<Element<'a>>,
    ) -> widget::MouseArea<'a, Message, LauncherTheme> {
        widget::mouse_area(elem).on_right_press(
            MainMenuMessage::Modal(Some(LaunchModal::SidebarCtxMenu(
                Some(selection.clone()),
                self.window_state.mouse_pos,
            )))
            .into(),
        )
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

fn drag_drop_receiver(
    menu: &MenuLaunch,
    selection: &SidebarSelection,
    node: &SidebarNode,
    nesting: i16,
) -> Option<widget::Column<'static, Message, LauncherTheme>> {
    if nesting == -1 {
        return None;
    }
    let Some(LaunchModal::Dragging { dragged_to, .. }) = &menu.modal else {
        return None;
    };

    let clickbox = |offset, elem| {
        let hover = |entered| {
            MainMenuMessage::DragHover {
                entered,
                location: SDragLocation {
                    offset,
                    sel: selection.clone(),
                },
            }
            .into()
        };

        widget::mouse_area(elem)
            .on_press(
                MainMenuMessage::DragDrop(Some(SDragLocation {
                    offset,
                    sel: selection.clone(),
                }))
                .into(),
            )
            .on_enter(hover(true))
            .on_exit(hover(false))
    };

    let empty = || widget::Space::new(Length::Fill, Length::Fill);
    let bar = || {
        widget::horizontal_rule(2).style(|t: &LauncherTheme| t.style_rule(Color::SecondLight, 4))
    };

    let (is_hovered, offset) = dragged_to
        .as_ref()
        .map(|n| (n.sel == *selection, n.offset))
        .unwrap_or((false, false));

    Some(
        widget::column![clickbox(
            false,
            widget::Column::new()
                .push_maybe((is_hovered && !offset).then_some(bar()))
                .push(empty())
        ),]
        .push_maybe(
            node.kind.show_bottom_target().then_some(clickbox(
                true,
                widget::Column::new()
                    .push(empty())
                    .push_maybe((is_hovered && offset).then_some(bar())),
            )),
        ),
    )
}

impl SidebarNodeKind {
    fn show_bottom_target(&self) -> bool {
        if let SidebarNodeKind::Folder {
            children,
            is_expanded,
            ..
        } = self
        {
            !*is_expanded || children.is_empty()
        } else {
            true
        }
    }
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
