use iced::{widget, Alignment, Length};

use crate::{
    menu_renderer::{back_button, Element},
    state::{EditLwjglMessage, MenuEditLwjgl, Message},
    stylesheet::styles::LauncherTheme,
};

impl MenuEditLwjgl {
    pub fn view(&'_ self, tick_timer: usize) -> Element<'_> {
        match self {
            MenuEditLwjgl::Loading { .. } => {
                let dots = ".".repeat((tick_timer % 3) + 1);

                widget::column![
                    back_button().on_press(Message::EditLwjgl(EditLwjglMessage::Back)),
                    widget::text!("Loading LWJGL version list from Maven Central{dots}",).size(20)
                ]
                .padding(10)
                .into()
            }
            MenuEditLwjgl::Loaded {
                versions,
                selected_version,
                initial_version,
                is_applying,
                mismatch_confirm,
            } => {
                let has_changed = Some(selected_version.as_str()) != initial_version.as_deref();

                // Combine all versions into one list for the picker
                let mut all_versions: Vec<String> = Vec::new();
                all_versions.extend(versions.lwjgl3.iter().cloned());
                all_versions.extend(versions.lwjgl2.iter().cloned());

                let warning_box: Element<'_> = if let Some(msg) = mismatch_confirm {
                    widget::container(
                        widget::column![
                            widget::text("⚠️  LWJGL mismatch")
                                .size(14)
                                .shaping(widget::text::Shaping::Advanced)
                                .color([1.0, 0.6, 0.0]),
                            widget::Space::with_height(5),
                            widget::text(msg).size(12),
                            widget::Space::with_height(10),
                            widget::row![
                                widget::button(widget::text("I know what I am doing").size(14))
                                    .on_press_maybe(if !is_applying {
                                        Some(Message::EditLwjgl(EditLwjglMessage::MismatchProceed))
                                    } else {
                                        None
                                    }),
                                widget::Space::with_width(10),
                                widget::button(widget::text("Revert").size(14)).on_press_maybe(
                                    if !is_applying {
                                        Some(Message::EditLwjgl(EditLwjglMessage::MismatchRevert))
                                    } else {
                                        None
                                    }
                                ),
                            ]
                            .align_y(Alignment::Center),
                        ]
                        .spacing(2),
                    )
                    .padding(10)
                    .style(|_theme: &LauncherTheme| widget::container::Style {
                        background: Some(iced::Color::from_rgb(0.2, 0.15, 0.1).into()),
                        border: iced::Border {
                            color: iced::Color::from_rgb(1.0, 0.6, 0.0),
                            width: 1.0,
                            radius: 4.0.into(),
                        },
                        ..Default::default()
                    })
                    .into()
                } else {
                    widget::container(
                        widget::column![
                            widget::text("⚠️  Warning: LWJGL 2.x and 3.x are NOT compatible!")
                                .size(14)
                                .shaping(widget::text::Shaping::Advanced)
                                .color([1.0, 0.6, 0.0]),
                            widget::Space::with_height(5),
                            widget::text("• Minecraft 1.12.2 and below use LWJGL 2.x").size(12),
                            widget::text("• Minecraft 1.13+ use LWJGL 3.x").size(12),
                            widget::text("• Using the wrong version will crash the game").size(12),
                        ]
                        .spacing(2),
                    )
                    .padding(10)
                    .style(|_theme: &LauncherTheme| widget::container::Style {
                        background: Some(iced::Color::from_rgb(0.2, 0.15, 0.1).into()),
                        border: iced::Border {
                            color: iced::Color::from_rgb(1.0, 0.6, 0.0),
                            width: 1.0,
                            radius: 4.0.into(),
                        },
                        ..Default::default()
                    })
                    .into()
                };

                widget::column![
                    back_button().on_press(Message::EditLwjgl(EditLwjglMessage::Back)),
                    widget::text("Change LWJGL Version").size(24),
                    widget::container(
                        widget::column![
                            widget::text("Select LWJGL Version:").size(16),
                            widget::pick_list(all_versions, Some(selected_version.clone()), |v| {
                                Message::EditLwjgl(EditLwjglMessage::VersionSelected(Some(v)))
                            })
                            .width(Length::Fixed(300.0)),
                            widget::Space::with_height(10),
                            warning_box,
                        ]
                        .spacing(10)
                    )
                    .padding(20),
                    widget::Space::with_height(20),
                    widget::row![
                        widget::button(if has_changed {
                            widget::text("Apply").size(16)
                        } else {
                            widget::text("Apply (no changes)").size(16)
                        })
                        .on_press_maybe(
                            if has_changed && !is_applying && mismatch_confirm.is_none() {
                                Some(Message::EditLwjgl(EditLwjglMessage::Apply))
                            } else {
                                None
                            }
                        ),
                        widget::Space::with_width(10),
                        widget::text(if *is_applying {
                            "Applying changes..."
                        } else if has_changed {
                            "Current: "
                        } else {
                            "No changes"
                        })
                        .size(14)
                        .color(if has_changed {
                            [0.7, 0.7, 0.7]
                        } else {
                            [0.5, 0.5, 0.5]
                        })
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center)
                ]
                .padding(10)
                .spacing(10)
                .into()
            }
        }
    }
}
