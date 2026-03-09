use iced::{widget, Length};

use crate::{
    icons,
    menu_renderer::{back_button, back_to_launch_screen, button_with_icon, Element},
    state::{MenuCloneInstance, Message},
};

impl MenuCloneInstance {
    pub fn view(&'_ self, tick_timer: usize) -> Element<'_> {
        widget::column![
            back_button().on_press(back_to_launch_screen(None, None)),
            "Select the contents of the instance you want to clone",
            "NOTE: Jarmods and other things will be copied outside of the scope of this selection menu.",
            widget::scrollable(if let Some(entries) = &self.entries {
                widget::column(entries.iter().enumerate().map(|(i, (entry, enabled))| {
                    let name = if entry.is_file {
                        entry.name.clone()
                    } else {
                        format!("{}/", entry.name)
                    };
                    widget::checkbox(name, *enabled)
                        .on_toggle(move |t| Message::CloneInstanceToggleItem(i, t))
                        .into()
                }))
                .padding(5)
            } else {
                let dots = ".".repeat((tick_timer % 3) + 1);
                widget::column!(widget::text!("Loading{dots}"))
            })
            .width(Length::Fill)
            .height(Length::Fill),
            button_with_icon(icons::floppydisk(), "Clone", 16)
                .on_press(Message::CloneInstanceStart),
        ]
        .padding(10)
        .spacing(10)
        .into()
    }
}
