use std::collections::HashSet;

use iced::{
    Alignment, Length,
    widget::{self, column, row},
};
use ql_mod_manager::store::SelectedMod;

use crate::{
    icons,
    menu_renderer::{Element, FONT_MONO, back_button, barthin, button_with_icon, tsubtitle},
    state::{
        EditPresetsMessage, ImageState, ManageModsMessage, MenuEditPresets, MenuRecommendedMods,
        Message, ModListEntry, SelectedState,
    },
    stylesheet::{color::Color, styles::LauncherTheme},
};

impl MenuEditPresets {
    pub fn view<'a>(&'a self, images: &'a ImageState) -> Element<'a> {
        match self {
            MenuEditPresets::Loading(t) => widget::column![widget::text(*t).size(20)]
                .padding(10)
                .into(),
            MenuEditPresets::Installing(progress) => widget::column!(
                widget::text("Importing Mods...").size(20),
                progress.view(),
                widget::text("Check debug log (at the bottom) for more info")
                    .size(12)
                    .style(tsubtitle),
            )
            .padding(10)
            .spacing(10)
            .into(),
            MenuEditPresets::Selecting {
                selected_mods,
                selected_state,
                sorted_mods_list,
                mc_dir_entries,
                include_config,
                drag_and_drop_hovered,
            } => {
                let p_main = column![
                    top_bar(),
                    widget::horizontal_rule(1).style(barthin),
                    column![
                        get_format_selector(),
                        widget::horizontal_rule(1).style(barthin),
                        row![
                            widget::text("Mods"),
                            select_all_button(*selected_state),
                            widget::horizontal_space(),
                            widget::checkbox(
                                "Include mod settings/configuration\n(config folder)",
                                *include_config
                            )
                            .text_size(12)
                            .on_toggle(|t| EditPresetsMessage::ToggleIncludeConfig(t).into()),
                        ]
                        .spacing(10)
                        .align_y(Alignment::Center)
                    ]
                    .spacing(10)
                    .padding(10),
                    Self::get_mods_list(sorted_mods_list, selected_mods, images),
                ];

                if *drag_and_drop_hovered {
                    widget::stack!(
                        p_main,
                        widget::center(widget::button(
                            widget::text("Drag and drop mod files to add them").size(20)
                        ))
                    )
                    .into()
                } else {
                    p_main.into()
                }
            }
        }
    }

    fn get_mods_list<'a>(
        sorted_mods_list: &'a [ModListEntry],
        selected_mods: &'a HashSet<SelectedMod>,
        images: &'a ImageState,
    ) -> Element<'a> {
        widget::responsive(|size| {
            widget::scrollable(
                widget::column(
                    sorted_mods_list
                        .chunks((size.width / 250.0).floor().max(1.0) as usize)
                        .map(|chunk| {
                            column![
                                widget::row(chunk.iter().map(|entry| {
                                    get_mod_entry(selected_mods, images, entry)
                                    // widget::container("").width(250).into()
                                }))
                                .spacing(10)
                                .width(Length::Fill),
                                widget::horizontal_rule(1).style(barthin)
                            ]
                            .spacing(5)
                            .into()
                        }),
                )
                .padding(20)
                .spacing(5),
            )
            .style(|t: &LauncherTheme, s| t.style_scrollable_flat_extra_dark(s))
            .height(Length::Fill)
            .into()
        })
        .into()
    }
}

fn select_all_button(
    selected_state: SelectedState,
) -> widget::Button<'static, Message, LauncherTheme> {
    widget::button(
        widget::text(if let SelectedState::All = selected_state {
            "Unselect All"
        } else {
            "Select All"
        })
        .size(14),
    )
    .on_press(EditPresetsMessage::SelectAll.into())
}

fn get_format_selector() -> Element<'static> {
    let format_radio = |n, v| {
        widget::radio(n, v, Some(true), |_| Message::Nothing)
            .size(14)
            .text_size(12)
            .spacing(5)
    };
    row![
        widget::text("Format:").size(14),
        // TODO
        format_radio("QuantumLauncher", true),
        format_radio("Modrinth", false),
        format_radio("Curseforge", false),
        format_radio("MultiMC/PrismLauncher", false),
    ]
    .align_y(Alignment::Center)
    .spacing(10)
    .wrap()
    .into()
}

fn top_bar() -> widget::Container<'static, Message, LauncherTheme> {
    widget::container(
        column![
            row![
                button_with_icon(icons::back_s(12), "Back", 13)
                    .padding([5, 8])
                    .on_press(ManageModsMessage::Open.into()),
                widget::text("Modpacks/Mod Presets...").width(Length::Fill),
                button_with_icon(icons::checkmark_s(12), "Build Preset", 13)
                    .padding([5, 8])
                    .on_press(EditPresetsMessage::Generate.into()),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            widget::text("Share your instance and mods setup with others through a single file!")
                .size(14)
                .style(tsubtitle),
        ]
        .spacing(5),
    )
    .style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::ExtraDark))
    .padding([5, 10])
}

fn get_mod_entry<'a>(
    selected_mods: &HashSet<SelectedMod>,
    images: &'a ImageState,
    entry: &'a ModListEntry,
) -> Element<'a> {
    if !entry.is_manually_installed() {
        return row![
            widget::text("(dependency)").size(12).style(tsubtitle),
            widget::text(entry.name())
                .size(14)
                .shaping(widget::text::Shaping::Advanced)
                .width(Length::Fill)
        ]
        .spacing(5)
        .into();
    }

    let is_checked = selected_mods.contains(&entry.clone().into());

    let checked_color = if is_checked {
        Color::Light
    } else {
        Color::SecondLight
    };

    match entry {
        ModListEntry::Downloaded { id, config } => {
            const ICON_SIZE: f32 = 20.0;

            let toggle =
                |t| EditPresetsMessage::ToggleCheckbox((config.name.clone(), id.clone()), t).into();
            widget::mouse_area(
                row![
                    widget::checkbox("", is_checked)
                        .size(14)
                        .on_toggle(toggle)
                        .spacing(0),
                    images.view(config.icon_url.as_deref(), Some(ICON_SIZE), Some(ICON_SIZE)),
                    widget::text(&config.name)
                        .size(14)
                        .style(move |t: &LauncherTheme| t.style_text(checked_color)),
                ]
                .spacing(10)
                .width(Length::Fill)
                .align_y(Alignment::Center),
            )
            .on_press(toggle(!is_checked))
            .into()
        }
        ModListEntry::Local { file_name } => widget::checkbox(file_name, is_checked)
            .font(FONT_MONO)
            .size(14)
            .text_size(14)
            .style(move |t: &LauncherTheme, s| t.style_checkbox(s, Some(checked_color)))
            .on_toggle(|t| EditPresetsMessage::ToggleCheckboxLocal(file_name.clone(), t).into())
            .width(Length::Fill)
            .into(),
    }
}

impl MenuRecommendedMods {
    pub fn view(&'_ self) -> Element<'_> {
        let back_button = back_button().on_press(ManageModsMessage::Open.into());

        match self {
            MenuRecommendedMods::Loading { progress, .. } => progress.view().padding(10).into(),
            MenuRecommendedMods::InstallALoader => {
                widget::column![
                    back_button,
                    "Install a mod loader (like Fabric/Forge/NeoForge/Quilt/etc, whichever is compatible)",
                    "You need one before you can install mods"
                ].padding(10).spacing(5).into()
            }
            MenuRecommendedMods::NotSupported => {
                widget::column![
                    back_button,
                    "No recommended mods found :)"
                ].padding(10).spacing(5).into()
            }
            MenuRecommendedMods::Loaded { mods, .. } => {
                let content: Element =
                    widget::column!(
                        back_button,
                        button_with_icon(icons::download(), "Download Recommended Mods", 16)
                            .on_press(crate::state::RecommendedModMessage::Download.into()),
                        widget::column(mods.iter().enumerate().map(|(i, (e, n))| {
                            let elem: Element = widget::checkbox(n.name, *e)
                                .on_toggle(move |n| {
                                    crate::state::RecommendedModMessage::Toggle(i, n).into()
                                })
                                .into();
                            widget::column!(
                                elem,
                                widget::text(n.description)
                                    .shaping(widget::text::Shaping::Advanced)
                                    .size(12)
                            )
                                .spacing(5)
                                .into()
                        }))
                        .spacing(10)
                    )
                    .spacing(10)
                    .into();

                widget::scrollable(widget::column![content].padding(10))
                    .style(|t: &LauncherTheme, status| t.style_scrollable_flat_dark(status))
                    .into()
            }
        }
    }
}
