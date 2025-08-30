use crate::{
    icon_manager,
    menu_renderer::{button_with_icon, FONT_MONO},
    state::{EditInstanceMessage, MenuEditInstance, Message},
    stylesheet::{color::Color, styles::LauncherTheme},
};
use iced::{widget, Length};
use ql_core::json::{
    instance_config::{JavaArgsMode, PreLaunchPrefixMode},
    GlobalSettings,
};
use ql_core::InstanceSelection;

use super::Element;

impl MenuEditInstance {
    pub fn view(&'_ self, selected_instance: &InstanceSelection) -> Element<'_> {
        let ts = |n: &LauncherTheme| n.style_text(Color::SecondLight);

        widget::scrollable(
            widget::column![
                widget::container(
                    widget::row![
                        widget::text(selected_instance.get_name().to_owned()).size(20).font(FONT_MONO),
                        widget::horizontal_space(),
                        widget::text!("{} | {}  ",
                            self.config.mod_type,
                            if selected_instance.is_server() {
                                "Server"
                            } else {
                                "Client"
                            }
                        )
                    ].padding(10).spacing(5),
                )
                .style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::Dark)),

                widget::container(widget::column!(
                    widget::row!(
                        widget::button("Rename").on_press(Message::EditInstance(EditInstanceMessage::RenameApply)),
                        widget::text_input("Rename Instance", &self.instance_name).on_input(|n| Message::EditInstance(EditInstanceMessage::RenameEdit(n))),
                    ).spacing(5),
                )
                .padding(10)
                .spacing(10))
                .style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::ExtraDark)),

                widget::container(
                    self.item_java_override()
                ).style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::Dark)),
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
                                false
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
            widget::column!(
                Self::get_java_args_list(
                    self.config.java_args.as_deref(),
                    |n| Message::EditInstance(EditInstanceMessage::JavaArgDelete(n)),
                    |n| Message::EditInstance(EditInstanceMessage::JavaArgShiftUp(n)),
                    |n| Message::EditInstance(EditInstanceMessage::JavaArgShiftDown(n)),
                    &|n, i| Message::EditInstance(EditInstanceMessage::JavaArgEdit(n, i))
                ),
                button_with_icon(icon_manager::create(), "Add", 16)
                    .on_press(Message::EditInstance(EditInstanceMessage::JavaArgsAdd))
            ),
            widget::text("Game arguments:").size(20),
            widget::column!(
                Self::get_java_args_list(
                    self.config.game_args.as_deref(),
                    |n| Message::EditInstance(EditInstanceMessage::GameArgDelete(n)),
                    |n| Message::EditInstance(EditInstanceMessage::GameArgShiftUp(n)),
                    |n| Message::EditInstance(EditInstanceMessage::GameArgShiftDown(n)),
                    &|n, i| Message::EditInstance(EditInstanceMessage::GameArgEdit(n, i))
                ),
                button_with_icon(icon_manager::create(), "Add", 16)
                    .on_press(Message::EditInstance(EditInstanceMessage::GameArgsAdd))
            ),
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
            widget::column!(
                Self::get_java_args_list(
                    self.config
                        .global_settings
                        .as_ref()
                        .and_then(|n| n.pre_launch_prefix.as_deref()),
                    |n| Message::EditInstance(EditInstanceMessage::PreLaunchPrefixDelete(n)),
                    |n| Message::EditInstance(EditInstanceMessage::PreLaunchPrefixShiftUp(n)),
                    |n| Message::EditInstance(EditInstanceMessage::PreLaunchPrefixShiftDown(n)),
                    &|n, i| Message::EditInstance(EditInstanceMessage::PreLaunchPrefixEdit(n, i))
                ),
                button_with_icon(icon_manager::create(), "Add", 16).on_press(
                    Message::EditInstance(EditInstanceMessage::PreLaunchPrefixAdd)
                )
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
            widget::text_input(
                "Leave blank if none",
                self.config.java_override.as_deref().unwrap_or_default()
            )
            .on_input(|t| Message::EditInstance(EditInstanceMessage::JavaOverride(t)))
        ]
        .padding(10)
        .spacing(10)
    }

    fn get_java_args_list<'a>(
        args: Option<&'a [String]>,
        mut msg_delete: impl FnMut(usize) -> Message,
        mut msg_up: impl FnMut(usize) -> Message,
        mut msg_down: impl FnMut(usize) -> Message,
        edit_msg: &'a dyn Fn(String, usize) -> Message,
    ) -> Element<'a> {
        const ITEM_SIZE: u16 = 10;

        let Some(args) = args else {
            return widget::column!().into();
        };
        widget::column(args.iter().enumerate().map(|(i, arg)| {
            widget::row!(
                widget::button(
                    widget::row![icon_manager::delete_with_size(ITEM_SIZE)]
                        .align_y(iced::Alignment::Center)
                        .padding(5)
                )
                .on_press(msg_delete(i)),
                widget::button(
                    widget::row![icon_manager::arrow_up_with_size(ITEM_SIZE)]
                        .align_y(iced::Alignment::Center)
                        .padding(5)
                )
                .on_press(msg_up(i)),
                widget::button(
                    widget::row![icon_manager::arrow_down_with_size(ITEM_SIZE)]
                        .align_y(iced::Alignment::Center)
                        .padding(5)
                )
                .on_press(msg_down(i)),
                widget::text_input("Enter argument...", arg)
                    .size(ITEM_SIZE + 8)
                    .on_input(move |n| edit_msg(n, i))
            )
            .into()
        }))
        .into()
    }
}

pub fn resolution_dialog<'a>(
    global_settings: Option<&GlobalSettings>,
    width: impl Fn(String) -> Message + 'a,
    height: impl Fn(String) -> Message + 'a,
    global: bool,
) -> widget::Column<'a, Message, LauncherTheme> {
    let ts = |n: &LauncherTheme| n.style_text(Color::SecondLight);

    widget::column![
        "Custom Game Window Size (px):",
        widget::text!(
            "The default size the Minecraft window will open in{}\n(Leave empty for default)",
            if global {
                "\nIndividual instances can override these settings."
            } else {
                ""
            }
        )
        .size(12)
        .style(ts),
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
        widget::text("Common resolutions: 854x480, 1366x768, 1920x1080, 2560x1440, 3840x2160")
            .size(12)
            .style(ts),
    ]
    .spacing(5)
}

pub fn global_java_args_dialog<'a>(
    java_args: Option<&'a [String]>,
    add_msg: Message,
    delete_msg: impl Fn(usize) -> Message + 'a,
    edit_msg: &'a dyn Fn(String, usize) -> Message,
    up_msg: impl Fn(usize) -> Message + 'a,
    down_msg: impl Fn(usize) -> Message + 'a,
) -> widget::Column<'a, Message, LauncherTheme> {
    let ts = |n: &LauncherTheme| n.style_text(Color::SecondLight);

    widget::column![
        "Global Java Arguments:",
        widget::text(
            r"These Java arguments will apply to all instances.
You can override or customize their behaviour on a per-instance basis too."
        )
        .size(12)
        .style(ts),
        widget::column!(
            MenuEditInstance::get_java_args_list(java_args, delete_msg, up_msg, down_msg, edit_msg),
            button_with_icon(icon_manager::create(), "Add Argument", 16).on_press(add_msg)
        )
        .spacing(5),
    ]
    .spacing(5)
}

pub fn global_pre_launch_prefix_dialog<'a>(
    prefix_args: Option<&'a [String]>,
    add_msg: Message,
    delete_msg: impl Fn(usize) -> Message + 'a,
    edit_msg: &'a dyn Fn(String, usize) -> Message,
    up_msg: impl Fn(usize) -> Message + 'a,
    down_msg: impl Fn(usize) -> Message + 'a,
) -> widget::Column<'a, Message, LauncherTheme> {
    let ts = |n: &LauncherTheme| n.style_text(Color::SecondLight);

    widget::column![
        "Global Pre-Launch Prefix:",
        widget::text(
            r"Commands to prepend to the game launch command.
Example: Use 'prime-run' to force NVIDIA GPU usage on Linux with Optimus graphics."
        )
        .size(12)
        .style(ts),
        widget::column!(
            MenuEditInstance::get_java_args_list(
                prefix_args,
                delete_msg,
                up_msg,
                down_msg,
                edit_msg
            ),
            button_with_icon(icon_manager::create(), "Add Command", 16).on_press(add_msg)
        )
        .spacing(5),
    ]
    .spacing(5)
}
