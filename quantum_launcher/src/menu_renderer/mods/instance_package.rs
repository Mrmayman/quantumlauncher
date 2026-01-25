use iced::{
    widget::{self, column},
    Length,
};

use crate::{
    icons,
    menu_renderer::{back_button, back_to_launch_screen, button_with_icon, ui::checkbox, Element},
    state::{MenuExportInstance, Message},
};

impl MenuExportInstance {
    pub fn view(&'_ self, tick_timer: usize) -> Element<'_> {
        column![
            back_button().on_press(back_to_launch_screen(None, None)),
            "Select the contents of the \".minecraft\" folder you want to keep",
            widget::scrollable(if let Some(entries) = &self.entries {
                widget::column(entries.iter().enumerate().map(|(i, (entry, enabled))| {
                    let name = if entry.is_file {
                        entry.name.clone()
                    } else {
                        format!("{}/", entry.name)
                    };
                    checkbox(widget::text(name), *enabled, move |t| {
                        Message::ExportInstanceToggleItem(i, t)
                    })
                }))
                .padding(5)
            } else {
                let dots = ".".repeat((tick_timer % 3) + 1);
                column!(widget::text!("Loading{dots}"))
            })
            .width(Length::Fill)
            .height(Length::Fill),
            column![
                widget::text("Format:").size(12),
                widget::row![
                    widget::pick_list(["QuantumLauncher"], Some("QuantumLauncher"), |_| {
                        Message::Nothing
                    })
                    .text_line_height(1.68),
                    button_with_icon(icons::floppydisk(), "Export", 16)
                        .on_press(Message::ExportInstanceStart),
                ]
                .spacing(5)
                .wrap()
            ]
            .spacing(2),
        ]
        .padding(10)
        .spacing(10)
        .into()
    }
}
