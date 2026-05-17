use iced::{
    Length,
    widget::{self, column},
};

use crate::{
    config::sidebar::{SDragLocation, SDragTo, SidebarNode, SidebarNodeKind, SidebarSelection},
    menu_renderer::sidebar::LEVEL_WIDTH,
    state::{MenuLaunch, Message, SidebarMessage},
    stylesheet::{color::Color, styles::LauncherTheme},
};

/// Renders the drag-and-drop guide lines (bright lines that show where you're dropping it)
pub fn drag_drop_receiver(
    menu: &MenuLaunch,
    selection: &SidebarSelection,
    node: &SidebarNode,
) -> Option<widget::Column<'static, Message, LauncherTheme>> {
    let (_, dragged_to) = menu.get_modal_drag()?;

    let (is_hovered, offset) = dragged_to.as_ref().map_or((false, SDragTo::Before), |n| {
        (n.sel == *selection, n.offset)
    });

    Some(column![
        drop_box(
            SDragTo::Before,
            is_hovered && matches!(offset, SDragTo::Before),
            selection
        ),
        bottom_drop_box(
            node,
            is_hovered && matches!(offset, SDragTo::After | SDragTo::Inside),
            selection,
        )
    ])
}

fn bottom_drop_box(
    node: &SidebarNode,
    show: bool,
    selection: &SidebarSelection,
) -> Option<widget::MouseArea<'static, Message, LauncherTheme>> {
    if let SidebarNodeKind::Folder(f) = &node.kind {
        return f
            .children
            .is_empty()
            .then(|| drop_box(SDragTo::Inside, show, selection));
    }
    Some(drop_box(SDragTo::After, show, selection))
}

fn drop_box<'a>(
    offset: SDragTo,
    show: bool,
    selection: &SidebarSelection,
) -> widget::MouseArea<'a, Message, LauncherTheme> {
    let hover = |entered| {
        SidebarMessage::DragHover {
            entered,
            location: SDragLocation {
                offset,
                sel: selection.clone(),
            },
        }
        .into()
    };

    let elem = show.then_some(bar(4));
    widget::mouse_area(match offset {
        SDragTo::Before => column![elem, empty()],
        SDragTo::After => column![empty(), elem],
        SDragTo::Inside => column![
            empty(),
            show.then(|| widget::row![
                widget::space().width(LEVEL_WIDTH).height(Length::Fill),
                bar(12)
            ]),
        ],
    })
    .on_press(
        SidebarMessage::DragDrop(Some(SDragLocation {
            offset,
            sel: selection.clone(),
        }))
        .into(),
    )
    .on_enter(hover(true))
    .on_exit(hover(false))
}

fn empty() -> widget::Space {
    widget::space().width(Length::Fill).height(Length::Fill)
}

fn bar(thickness: u32) -> widget::Rule<'static, LauncherTheme> {
    widget::rule::horizontal(thickness).style(|t: &LauncherTheme| t.style_rule(Color::SecondLight))
}
