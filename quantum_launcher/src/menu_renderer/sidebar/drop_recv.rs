use iced::{
    widget::{self, column},
    Length,
};

use crate::{
    config::sidebar::{SDragLocation, SDragTo, SidebarNode, SidebarNodeKind, SidebarSelection},
    menu_renderer::sidebar::LEVEL_WIDTH,
    state::{MainMenuMessage, MenuLaunch, Message},
    stylesheet::{color::Color, styles::LauncherTheme},
};

pub fn drag_drop_receiver(
    menu: &MenuLaunch,
    selection: &SidebarSelection,
    node: &SidebarNode,
) -> Option<widget::Column<'static, Message, LauncherTheme>> {
    let (_, dragged_to) = menu.get_modal_drag()?;

    let (is_hovered, offset) = dragged_to
        .as_ref()
        .map(|n| (n.sel == *selection, n.offset))
        .unwrap_or((false, SDragTo::Before));

    Some(
        column![drop_box(
            SDragTo::Before,
            is_hovered && matches!(offset, SDragTo::Before),
            selection
        ),]
        .push_maybe(bottom_drop_box(
            selection,
            node,
            is_hovered && matches!(offset, SDragTo::After | SDragTo::Inside),
        )),
    )
}

fn bottom_drop_box(
    selection: &SidebarSelection,
    node: &SidebarNode,
    show: bool,
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
        MainMenuMessage::DragHover {
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
        SDragTo::Before => widget::Column::new().push_maybe(elem).push(empty()),
        SDragTo::After => widget::column![empty()].push_maybe(elem),
        SDragTo::Inside => widget::column![empty()].push_maybe(
            show.then(|| widget::row![widget::Space::new(LEVEL_WIDTH, Length::Fill), bar(12)]),
        ),
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

fn bar(thickness: u16) -> widget::Rule<'static, LauncherTheme> {
    widget::horizontal_rule(thickness)
        .style(move |t: &LauncherTheme| t.style_rule(Color::SecondLight, thickness))
}
