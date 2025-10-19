use crate::{
    icon_manager,
    menu_renderer::{button_with_icon, FONT_MONO},
    state::{
        CustomJarState, EditInstanceMessage, ListMessage, MenuEditInstance, Message, NONE_JAR_NAME,
    },
    stylesheet::{color::Color, styles::LauncherTheme, widgets::StyleButton},
};
use iced::{widget, Length};
use ql_core::json::{
    instance_config::{JavaArgsMode, PreLaunchPrefixMode},
    GlobalSettings,
};
use ql_core::InstanceSelection;

use super::Element;

impl MenuEditInstance {
    pub fn view<'a>(
        &'a self,
        selected_instance: &InstanceSelection,
        jar_choices: Option<&'a CustomJarState>,
    ) -> Element<'a> {
        let ts = |n: &LauncherTheme| n.style_text(Color::SecondLight);

        widget::scrollable(
            widget::column![
                widget::container(
                    widget::column![
                        widget::row![
                            widget::text(selected_instance.get_name().to_owned()).size(20).font(FONT_MONO),
                        ].push_maybe((!self.is_editing_name).then_some(
                            widget::button(
                                icon_manager::edit_with_size(12)
                                    .style(|t: &LauncherTheme| t.style_text(Color::Mid))
                            ).style(|t: &LauncherTheme, s|
                                t.style_button(s, StyleButton::FlatDark)
                            )
                            .on_press(Message::EditInstance(EditInstanceMessage::RenameToggle))
                        )).spacing(5),

                        widget::text!("{} {}",
                            self.config.mod_type,
                            if selected_instance.is_server() {
                                "Server"
                            } else {
                                "Client"
                            }
                        ).style(|t: &LauncherTheme| t.style_text(Color::Mid)).size(14),
                    ].padding(10).spacing(5).push_maybe(self.is_editing_name.then_some(widget::column![
                        widget::Space::with_height(1),
                        widget::text_input("Rename Instance", &self.instance_name).on_input(|n| Message::EditInstance(EditInstanceMessage::RenameEdit(n))),
                        widget::row![
                            widget::button(widget::text("Rename").size(12)).on_press(Message::EditInstance(EditInstanceMessage::RenameApply)),
                            widget::button(widget::text("Cancel").size(12)).on_press(Message::EditInstance(EditInstanceMessage::RenameToggle))
                        ].spacing(5)
                    ].spacing(5))),
                ).width(Length::Fill)
                .style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::Dark)),

                widget::container(
                    self.item_java_override()
                ).style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::ExtraDark)),
                widget::container(
                    self.item_custom_jar(jar_choices)
                ).width(Length::Fill).style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::Dark)),
                widget::container(
                    self.item_mem_alloc(),
                ).style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::ExtraDark)),
                widget::container(
                    widget::Column::new()
                    .push_maybe((!selected_instance.is_server()).then_some(widget::column![
                        widget::checkbox("Close launcher after game opens", self.config.close_on_start.unwrap_or(false))
                            .on_toggle(|t| Message::EditInstance(EditInstanceMessage::CloseLauncherToggle(t))),
                    ].spacing(5)))
                    .push(
                        widget::column![
                            widget::Space::with_height(5),
                            widget::checkbox("DEBUG: Enable log system (recommended)", self.config.enable_logger.unwrap_or(true))
                                .on_toggle(|t| Message::EditInstance(EditInstanceMessage::LoggingToggle(t))),
                            widget::text("Once disabled, logs will be printed in launcher STDOUT.\nRun the launcher executable from the terminal/command prompt to see it").size(12).style(ts),
                            widget::horizontal_space(),
                        ].spacing(5)
                    )
                    .padding(10)
                    .spacing(10)
                ).style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::Dark)),
                widget::container(
                    self.item_args()
                ).style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::ExtraDark)),
                widget::container(
                    widget::Column::new()
                        .push_maybe((!selected_instance.is_server()).then_some(
                            resolution_dialog(
                                self.config.global_settings.as_ref(),
                                |n| Message::EditInstance(EditInstanceMessage::WindowWidthChanged(n)),
                                |n| Message::EditInstance(EditInstanceMessage::WindowHeightChanged(n)),
                        )))
                )
                .style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::Dark))
                .padding(10)
                .width(Length::Fill),
                widget::container(
                    button_with_icon(icon_manager::delete(), "Delete Instance", 16)
                        .on_press(Message::DeleteInstanceMenu)
                )
                .width(Length::Fill)
                .padding(10)
                .style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::ExtraDark)),
            ]
        ).style(LauncherTheme::style_scrollable_flat_extra_dark).into()
    }

    fn item_args(&self) -> widget::Column<'_, Message, LauncherTheme> {
        let current_mode = self.config.java_args_mode.unwrap_or_default();

        widget::column!(
            widget::container(
                widget::column![
                    widget::text("Interaction with global arguments:").size(14),
                    widget::pick_list(JavaArgsMode::ALL, Some(current_mode), |mode| {
                        Message::EditInstance(EditInstanceMessage::JavaArgsModeChanged(mode))
                    })
                    .placeholder("Select mode...")
                    .width(150)
                    .text_size(14),
                    Self::get_mode_description(current_mode),
                ]
                .padding(10)
                .spacing(7)
            ),
            widget::text("Java arguments:").size(20),
            get_args_list(self.config.java_args.as_deref(), |n| Message::EditInstance(
                EditInstanceMessage::JavaArgs(n)
            )),
            widget::text("Game arguments:").size(20),
            get_args_list(self.config.game_args.as_deref(), |n| Message::EditInstance(
                EditInstanceMessage::GameArgs(n)
            )),
            widget::text("Pre-launch prefix:").size(20),
            widget::container(
                widget::column![
                    widget::text("Interaction with global pre-launch prefix:").size(14),
                    widget::pick_list(
                        PreLaunchPrefixMode::ALL,
                        Some(self.config.pre_launch_prefix_mode.unwrap_or_default()),
                        |mode| {
                            Message::EditInstance(EditInstanceMessage::PreLaunchPrefixModeChanged(
                                mode,
                            ))
                        }
                    )
                    .placeholder("Select mode...")
                    .width(200)
                    .text_size(14),
                    Self::get_prefix_mode_description(
                        self.config.pre_launch_prefix_mode.unwrap_or_default()
                    ),
                ]
                .padding(10)
                .spacing(7)
            ),
            get_args_list(
                self.config
                    .global_settings
                    .as_ref()
                    .and_then(|n| n.pre_launch_prefix.as_deref()),
                |n| Message::EditInstance(EditInstanceMessage::PreLaunchPrefix(n)),
            ),
        )
        .padding(10)
        .spacing(10)
        .width(Length::Fill)
    }

    fn get_mode_description<'a>(mode: JavaArgsMode) -> widget::Text<'a, LauncherTheme> {
        let description = mode.get_description();

        widget::text(description)
            .size(12)
            .style(|theme: &LauncherTheme| theme.style_text(Color::SecondLight))
    }

    fn get_prefix_mode_description<'a>(
        mode: PreLaunchPrefixMode,
    ) -> widget::Text<'a, LauncherTheme> {
        let description = mode.get_description();

        widget::text(description)
            .size(12)
            .style(|theme: &LauncherTheme| theme.style_text(Color::SecondLight))
    }

    fn item_mem_alloc(&self) -> widget::Column<'_, Message, LauncherTheme> {
        // 2 ^ 8 = 256 MB
        const MEM_256_MB_IN_TWOS_EXPONENT: f32 = 8.0;
        // 2 ^ 13 = 8192 MB
        const MEM_8192_MB_IN_TWOS_EXPONENT: f32 = 13.0;

        let ts = |n: &LauncherTheme| n.style_text(Color::SecondLight);

        widget::column![
            "Allocated memory",
            widget::text("For normal Minecraft, allocate 2 - 3 GB")
                .size(12)
                .style(ts),
            widget::text("For old versions, allocate 512 MB - 1 GB")
                .size(12)
                .style(ts),
            widget::text("For heavy modpacks/very high render distances, allocate 4 - 8 GB")
                .size(12)
                .style(ts),
            widget::slider(
                MEM_256_MB_IN_TWOS_EXPONENT..=MEM_8192_MB_IN_TWOS_EXPONENT,
                self.slider_value,
                |n| Message::EditInstance(EditInstanceMessage::MemoryChanged(n))
            )
            .step(0.1),
            widget::text(&self.slider_text),
        ]
        .padding(10)
        .spacing(5)
    }

    fn item_java_override(&self) -> widget::Column<'_, Message, LauncherTheme> {
        widget::column![
            "Custom Java executable (full path)",
            widget::text("Note: The launcher already sets up Java automatically,\nYou won't need this in most cases").size(12),
            widget::text_input(
                "Leave blank if none",
                self.config.java_override.as_deref().unwrap_or_default()
            )
            .on_input(|t| Message::EditInstance(EditInstanceMessage::JavaOverride(t)))
        ]
        .padding(10)
        .spacing(10)
    }

    fn item_custom_jar<'a>(
        &'a self,
        jar_choices: Option<&'a CustomJarState>,
    ) -> widget::Column<'a, Message, LauncherTheme> {
        let ts = |n: &LauncherTheme| n.style_text(Color::SecondLight);

        let picker: Element = if let Some(choices) = jar_choices {
            widget::pick_list(
                choices.choices.as_slice(),
                Some(
                    self.config
                        .custom_jar
                        .as_ref()
                        .map(|n| n.name.clone())
                        .unwrap_or(NONE_JAR_NAME.to_owned()),
                ),
                |t| Message::EditInstance(EditInstanceMessage::CustomJarPathChanged(t)),
            )
            .into()
        } else {
            "Loading...".into()
        };

        widget::column![
            "Custom JAR file",
            widget::text(
                r#"This feature is for *replacing* the Minecraft JAR,
not adding to it.
If you want to apply tweaks to your existing JAR file,
use "Mods->Jar Mods""#
            )
            .size(12)
            .style(ts),
            widget::Space::with_height(2),
            picker,
            widget::Column::new().push_maybe(
                self.config.custom_jar.is_some().then_some(
                    widget::column![
                        widget::text("Try this in case the game crashes otherwise")
                            .size(12)
                            .style(ts),
                        widget::checkbox(
                            "Auto-set mainClass",
                            self.config
                                .custom_jar
                                .as_ref()
                                .is_some_and(|n| n.autoset_main_class)
                        )
                        .on_toggle(|n| Message::EditInstance(
                            EditInstanceMessage::AutoSetMainClassToggle(n)
                        )),
                        widget::Space::with_height(5),
                    ]
                    .spacing(5)
                )
            ),
        ]
        .padding(10)
        .spacing(5)
    }
}

pub fn resolution_dialog<'a>(
    global_settings: Option<&GlobalSettings>,
    width: impl Fn(String) -> Message + 'a,
    height: impl Fn(String) -> Message + 'a,
) -> widget::Column<'a, Message, LauncherTheme> {
    let ts = |n: &LauncherTheme| n.style_text(Color::SecondLight);

    widget::column![
        "Custom Game Window Size (px):",
        widget::text("(Leave empty for default)\nCommon resolutions: 854x480, 1366x768, 1920x1080, 2560x1440, 3840x2160").size(12).style(ts),
        widget::row![
            widget::text("Width:").size(14),
            widget::text_input(
                "854",
                &global_settings
                    .and_then(|n| n.window_width)
                    .map_or(String::new(), |w| w.to_string())
            )
            .on_input(width)
            .width(100),
            widget::text("Height:").size(14),
            widget::text_input(
                "480",
                &global_settings
                    .and_then(|n| n.window_height)
                    .map_or(String::new(), |h| h.to_string())
            )
            .on_input(height)
            .width(100),
        ]
        .spacing(10)
        .align_y(iced::alignment::Vertical::Center),
    ]
    .spacing(5)
}

pub fn get_args_list<'a>(
    args: Option<&'a [String]>,
    msg: impl Fn(ListMessage) -> Message + Clone + 'static,
) -> Element<'a> {
    const ITEM_SIZE: u16 = 10;

    let Some(args) = args else {
        return widget::column!().into();
    };
    widget::column![
        widget::column(args.iter().enumerate().map(|(i, arg)| {
            widget::row!(
                widget::button(
                    widget::row![icon_manager::delete_with_size(ITEM_SIZE)]
                        .align_y(iced::Alignment::Center)
                        .padding(5)
                )
                .on_press(msg(ListMessage::Delete(i))),
                widget::button(
                    widget::row![icon_manager::arrow_up_with_size(ITEM_SIZE)]
                        .align_y(iced::Alignment::Center)
                        .padding(5)
                )
                .on_press(msg(ListMessage::ShiftUp(i))),
                widget::button(
                    widget::row![icon_manager::arrow_down_with_size(ITEM_SIZE)]
                        .align_y(iced::Alignment::Center)
                        .padding(5)
                )
                .on_press(msg(ListMessage::ShiftDown(i))),
                widget::text_input("Enter argument...", arg)
                    .size(ITEM_SIZE + 8)
                    .on_input({
                        let msg = msg.clone();
                        move |n| msg(ListMessage::Edit(n, i))
                    })
            )
            .into()
        })),
        button_with_icon(icon_manager::create(), "Add", 16).on_press(msg(ListMessage::Add))
    ]
    .spacing(5)
    .into()
}
