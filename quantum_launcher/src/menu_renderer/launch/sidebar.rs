use iced::{
    widget::{self, column, row},
    Alignment, Length,
};
use ql_core::InstanceSelection;

use crate::{
    config::sidebar::{InstanceKind, SidebarNode, SidebarNodeKind, SidebarSelection},
    menu_renderer::{tsubtitle, underline_maybe, Element},
    state::{LaunchModal, Launcher, MainMenuMessage, MenuLaunch, Message},
    stylesheet::{color::Color, styles::LauncherTheme, widgets::StyleButton},
};

impl Launcher {
    pub(super) fn get_node_rendered<'a>(
        &'a self,
        menu: &'a MenuLaunch,
        node: &'a SidebarNode,
        nesting: u16,
    ) -> Element<'a> {
        const LEVEL_WIDTH: u16 = 15;

        let text = widget::text(&node.name).size(15).style(tsubtitle);

        let nesting_inner = widget::Space::with_width(LEVEL_WIDTH * nesting);
        let nesting_outer = move |c| {
            widget::row((0..nesting).into_iter().map(|_: u16| {
                row![
                    widget::Space::with_width(LEVEL_WIDTH - 2),
                    widget::vertical_rule(1).style(move |t: &LauncherTheme| t.style_rule(c, 1))
                ]
                .into()
            }))
        };

        let drag_handle = |selection: SidebarSelection| {
            widget::mouse_area(
                widget::row![widget::text("=")
                    .size(20)
                    .style(|t: &LauncherTheme| t.style_text(Color::ExtraDark))]
                .padding([0, 4])
                .align_y(Alignment::Center),
            )
            .on_press(Message::Nothing)
        }; // TODO, `selection` variable will be used here

        match &node.kind {
            SidebarNodeKind::Instance(_) => {
                let is_selected = self.node_is_instance_selected(node);
                // Tbh should be careful about careless heap allocations
                let selection = SidebarSelection::Instance(
                    node.name.clone(),
                    if menu.is_viewing_server {
                        InstanceKind::Server
                    } else {
                        InstanceKind::Client
                    },
                );

                let button = node_button(
                    row![widget::Space::with_width(2), nesting_inner, text]
                        .push_maybe(self.get_running_icon(menu, &node.name)),
                )
                .on_press_maybe((!is_selected).then(|| {
                    MainMenuMessage::InstanceSelected(InstanceSelection::new(
                        &node.name,
                        menu.is_viewing_server,
                    ))
                    .into()
                }));

                let entry = self.node_get_button(is_selected, &selection, button);

                widget::stack!(
                    entry,
                    widget::row![widget::horizontal_space(), drag_handle(selection)],
                    nesting_outer(if is_selected {
                        Color::Mid
                    } else {
                        Color::SecondDark
                    }),
                )
                .into()
            }
            SidebarNodeKind::Folder {
                id,
                children,
                is_expanded,
            } => {
                let selection = SidebarSelection::Folder(*id);

                let inner = row![
                    nesting_inner,
                    if *is_expanded { "v  " } else { ">  " },
                    text
                ];
                let folder = column![node_button(inner)
                    .padding([4, 10])
                    .on_press(MainMenuMessage::ToggleFolderVisibility(*id).into())]
                .push_maybe(is_expanded.then(|| {
                    widget::column(
                        children
                            .iter()
                            .map(|node| self.get_node_rendered(menu, node, nesting + 1)),
                    )
                }))
                .width(Length::Fill);

                let entry = self.node_get_button(false, &selection, folder);
                widget::stack!(
                    entry,
                    widget::row![widget::horizontal_space(), drag_handle(selection)],
                    nesting_outer(Color::SecondDark)
                )
                .into()
            }
        }
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
