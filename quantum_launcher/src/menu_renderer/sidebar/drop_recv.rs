use iced::{
    widget::{self, column},
    Length,
};

use crate::{
    config::sidebar::{SDragLocation, SidebarNode, SidebarNodeKind, SidebarSelection},
    menu_renderer::{sidebar::LEVEL_WIDTH, Element},
    state::{LaunchModal, MainMenuMessage, MenuLaunch, Message},
    stylesheet::{color::Color, styles::LauncherTheme},
};

pub fn drag_drop_receiver(
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

    let bar = || {
        widget::horizontal_rule(4).style(|t: &LauncherTheme| t.style_rule(Color::SecondLight, 4))
    };

    let (is_hovered, offset) = dragged_to
        .as_ref()
        .map(|n| (n.sel == *selection, n.offset))
        .unwrap_or((false, false));

    Some(
        column![drop_box(
            false,
            (is_hovered && !offset).then_some(bar()),
            selection
        ),]
        .push_maybe({
            let cb = || drop_box(true, (is_hovered && offset).then_some(bar()), selection);
            if let SidebarNodeKind::Folder {
                is_expanded,
                children,
                ..
            } = &node.kind
            {
                if *is_expanded {
                    if children.is_empty() {
                        Some(drop_box(
                            true,
                            (is_hovered && offset).then_some(widget::row![
                                widget::Space::with_width(LEVEL_WIDTH),
                                widget::horizontal_rule(12).style(|t: &LauncherTheme| {
                                    t.style_rule(Color::SecondLight, 12)
                                })
                            ]),
                            selection,
                        ))
                    } else {
                        None
                    }
                } else {
                    Some(cb())
                }
            } else {
                Some(cb())
            }
        }),
    )
}

fn drop_box<'a>(
    offset: bool,
    elem: Option<impl Into<Element<'a>>>,
    selection: &SidebarSelection,
) -> widget::MouseArea<'a, Message, LauncherTheme> {
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

    widget::mouse_area(if offset {
        widget::column![empty()].push_maybe(elem)
    } else {
        widget::Column::new().push_maybe(elem).push(empty())
    })
    .on_press(
        MainMenuMessage::DragDrop(Some(SDragLocation {
            offset,
            sel: selection.clone(),
        }))
        .into(),
    )
    .on_enter(hover(true))
    .on_exit(hover(false))
}

fn empty() -> widget::Space {
    widget::Space::new(Length::Fill, Length::Fill)
}
