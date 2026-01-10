use iced::{
    widget::{self, column, row},
    Alignment, Length,
};
use ql_core::InstanceSelection;

use crate::{
    config::sidebar::{SidebarNode, SidebarNodeKind, SidebarSelection},
    menu_renderer::{underline_maybe, Element},
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
        let is_being_dragged = if let Some(LaunchModal::Dragging(sel)) = &menu.modal {
            *sel == selection && nesting != DRAGGED_TOOLTIP
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

        let drag_handle = widget::mouse_area(
            widget::row![widget::text("=")
                .size(20)
                .style(|t: &LauncherTheme| t.style_text(Color::ExtraDark))]
            .padding([0, 4])
            .align_y(Alignment::Center),
        )
        .on_press(MainMenuMessage::Modal(Some(LaunchModal::Dragging(selection.clone()))).into());

        let button: Element = match &node.kind {
            SidebarNodeKind::Instance(_) => {
                let node_view = row![widget::Space::with_width(2), nesting_inner, text]
                    .push_maybe(self.get_running_icon(menu, &node.name));
                if nesting == DRAGGED_TOOLTIP {
                    widget::container(node_view)
                        .style(|t: &LauncherTheme| {
                            t.style_container_sharp_box(0.0, Color::ExtraDark)
                        })
                        .padding([5, 10])
                        .width(200)
                        .into()
                } else {
                    node_button(node_view)
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
                    widget::text(if *is_expanded { "v  " } else { ">  " }).style(
                        move |t: &LauncherTheme| t.style_text(if is_being_dragged {
                            Color::Mid
                        } else {
                            Color::Light
                        })
                    ),
                    text
                ];
                if nesting == DRAGGED_TOOLTIP {
                    column![widget::container(inner)
                        .style(|t: &LauncherTheme| {
                            t.style_container_sharp_box(0.0, Color::ExtraDark)
                        })
                        .padding([4, 10])
                        .width(200)]
                } else {
                    column![node_button(inner)
                        .padding([4, 10])
                        .on_press(MainMenuMessage::ToggleFolderVisibility(*id).into())]
                    .push_maybe(is_expanded.then(|| {
                        widget::column(
                            children
                                .iter()
                                .map(|node| self.get_node_rendered(menu, node, nesting + 1)),
                        )
                    }))
                    .width(Length::Fill)
                }
                .into()
            }
        };

        widget::stack!(
            self.node_get_button(is_selected, &selection, button,),
            widget::row![widget::horizontal_space(), drag_handle],
            nesting_outer(if is_selected {
                Color::Mid
            } else {
                Color::SecondDark
            }),
        )
        .into()
    }

    fn node_is_instance_selected(&self, node: &SidebarNode) -> bool {
        self.selected_instance
            .as_ref()
            .is_some_and(|sel| node == sel)
    }

    fn node_get_button<'a>(
        &self,
        is_selected: bool,
        selection: &SidebarSelection,
        elem: impl Into<Element<'a>>,
    ) -> widget::MouseArea<'a, Message, LauncherTheme> {
        widget::mouse_area(underline_maybe(elem, Color::Dark, !is_selected)).on_right_press(
            MainMenuMessage::Modal(Some(LaunchModal::SidebarCtxMenu(
                Some(selection.clone()),
                self.window_state.mouse_pos,
            )))
            .into(),
        )
    }
}

fn node_button<'a>(inner: impl Into<Element<'a>>) -> widget::Button<'a, Message, LauncherTheme> {
    widget::button(inner)
        .style(|n: &LauncherTheme, status| n.style_button(status, StyleButton::FlatExtraDark))
        .width(Length::Fill)
}
