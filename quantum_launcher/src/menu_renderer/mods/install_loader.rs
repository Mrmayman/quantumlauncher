use iced::{widget, Alignment, Length};
use ql_core::InstanceSelection;
use ql_mod_manager::loaders::fabric::{self, FabricVersionListItem};

use crate::{
    icon_manager,
    menu_renderer::{back_button, button_with_icon, Element},
    state::{
        InstallFabricMessage, InstallOptifineMessage, ManageModsMessage, MenuInstallFabric,
        MenuInstallForge, MenuInstallOptifine, Message,
    },
    stylesheet::styles::LauncherTheme,
};

impl MenuInstallOptifine {
    pub fn view(&'_ self) -> Element<'_> {
        match self {
            MenuInstallOptifine::InstallingB173 => {
                widget::column![widget::text("Installing OptiFine for Beta 1.7.3...").size(20)]
                    .padding(10)
            }
            MenuInstallOptifine::Installing {
                optifine_install_progress,
                java_install_progress,
                is_java_being_installed,
                ..
            } => widget::column!(
                widget::text("Installing OptiFine").size(20),
                optifine_install_progress.view()
            )
            .push_maybe(
                java_install_progress
                    .as_ref()
                    .filter(|_| *is_java_being_installed)
                    .map(|java| java.view()),
            )
            .padding(10)
            .spacing(10),
            MenuInstallOptifine::Choosing {
                delete_installer,
                drag_and_drop_hovered,
                ..
            } => {
                let menu = self
                    .install_optifine_screen(*delete_installer)
                    .padding(10)
                    .spacing(10);
                if *drag_and_drop_hovered {
                    widget::column![widget::stack!(
                        menu,
                        widget::center(widget::button(
                            widget::text("Drag and drop the OptiFine installer").size(20)
                        ))
                    )]
                } else {
                    menu
                }
            }
        }
        .into()
    }

    pub fn install_optifine_screen<'a>(
        &self,
        delete_installer: bool,
    ) -> widget::Column<'a, Message, LauncherTheme, iced::Renderer> {
        widget::column!(
            back_button().on_press(Message::ManageMods(
                ManageModsMessage::ScreenOpenWithoutUpdate
            )),
            widget::container(
                widget::column!(
                    widget::text("Install OptiFine").size(20),
                    "Step 1: Open the OptiFine download page and download the installer.",
                    "WARNING: Make sure to download the correct version.",
                    widget::button("Open download page")
                        .on_press(Message::CoreOpenLink(self.get_url().to_owned()))
                )
                .padding(10)
                .spacing(10)
            ),
            widget::container(
                widget::column!(
                    "Step 2: Select the installer file",
                    widget::checkbox("Delete installer after use", delete_installer).on_toggle(
                        |t| Message::InstallOptifine(
                            InstallOptifineMessage::DeleteInstallerToggle(t)
                        )
                    ),
                    widget::button("Select File").on_press(Message::InstallOptifine(
                        InstallOptifineMessage::SelectInstallerStart
                    ))
                )
                .padding(10)
                .spacing(10)
            )
        )
        .width(Length::Fill)
        .height(Length::Fill)
    }
}

impl MenuInstallFabric {
    pub fn view(&'_ self, selected_instance: &InstanceSelection, tick_timer: usize) -> Element<'_> {
        match self {
            MenuInstallFabric::Loading { is_quilt, .. } => {
                let loader_name = if *is_quilt { "Quilt" } else { "Fabric" };
                let dots = ".".repeat((tick_timer % 3) + 1);

                widget::column![
                    back_button().on_press(Message::ManageMods(
                        ManageModsMessage::ScreenOpenWithoutUpdate
                    )),
                    widget::text!("Loading {loader_name} version list{dots}",).size(20)
                ]
            }
            MenuInstallFabric::Loaded {
                progress: Some(progress),
                backend,
                ..
            } => {
                widget::column![
                    widget::text!("Installing {backend}...").size(20),
                    progress.view(),
                ]
            }
            MenuInstallFabric::Unsupported(is_quilt) => {
                widget::column!(
                    back_button().on_press(Message::ManageMods(
                        ManageModsMessage::ScreenOpenWithoutUpdate
                    )),
                    widget::text!(
                        "{} is unsupported for this Minecraft version.",
                        if *is_quilt { "Quilt" } else { "Fabric" }
                    )
                )
            }
            MenuInstallFabric::Loaded {
                fabric_versions: fabric::VersionList::Unsupported,
                backend,
                ..
            } => {
                widget::column!(
                    back_button().on_press(Message::ManageMods(
                        ManageModsMessage::ScreenOpenWithoutUpdate
                    )),
                    widget::text!("{backend} is unsupported for this Minecraft version.")
                )
            }
            MenuInstallFabric::Loaded {
                backend,
                fabric_version,
                fabric_versions,
                ..
            } => {
                let picker = match fabric_versions {
                    fabric::VersionList::Quilt(l)
                    | fabric::VersionList::Fabric(l)
                    | fabric::VersionList::LegacyFabric(l)
                    | fabric::VersionList::OrnitheMC(l) => version_list(&l, fabric_version),

                    fabric::VersionList::Beta173 {
                        ornithe_mc,
                        babric,
                        cursed_legacy,
                    } => {
                        let list = match backend {
                            fabric::BackendType::OrnitheMC => ornithe_mc,
                            fabric::BackendType::Babric => babric,
                            fabric::BackendType::CursedLegacy => cursed_legacy,
                            _ => unreachable!(),
                        };

                        widget::column![
                            "Pick an implementation of Fabric:",
                            widget::pick_list(
                                [
                                    fabric::BackendType::Babric,
                                    fabric::BackendType::OrnitheMC,
                                    fabric::BackendType::CursedLegacy
                                ],
                                Some(backend),
                                |b| Message::InstallFabric(InstallFabricMessage::ChangeBackend(b))
                            ),
                            version_list(list, fabric_version),
                        ]
                        .spacing(5)
                    }
                    fabric::VersionList::Both {
                        legacy_fabric,
                        ornithe_mc,
                    } => {
                        let list = match backend {
                            fabric::BackendType::LegacyFabric => legacy_fabric,
                            fabric::BackendType::OrnitheMC => ornithe_mc,
                            _ => unreachable!(),
                        };

                        widget::column![
                            "Pick an implementation of Fabric:",
                            widget::pick_list(
                                [
                                    fabric::BackendType::LegacyFabric,
                                    fabric::BackendType::OrnitheMC,
                                ],
                                Some(backend),
                                |b| Message::InstallFabric(InstallFabricMessage::ChangeBackend(b))
                            ),
                            version_list(list, fabric_version),
                        ]
                        .spacing(5)
                    }

                    fabric::VersionList::Unsupported => unreachable!(),
                };

                widget::column![
                    back_button().on_press(Message::ManageMods(
                        ManageModsMessage::ScreenOpenWithoutUpdate
                    )),
                    widget::text!("Install {backend} for \"{}\"", selected_instance.get_name())
                        .size(20),
                    picker,
                    button_with_icon(icon_manager::download(), "Install", 16)
                        .on_press(Message::InstallFabric(InstallFabricMessage::ButtonClicked)),
                ]
            }
        }
        .padding(10)
        .spacing(10)
        .into()
    }
}

fn version_list<'a>(
    list: &'a [FabricVersionListItem],
    selected: &'a str,
) -> widget::Column<'a, Message, LauncherTheme> {
    let selected = FabricVersionListItem {
        loader: fabric::FabricVersion {
            version: selected.to_owned(),
        },
    };
    widget::column![
        widget::text("Version:"),
        widget::row![widget::pick_list(list, Some(selected.clone()), |n| {
            Message::InstallFabric(InstallFabricMessage::VersionSelected(n.loader.version))
        })]
        .push_maybe(
            list.first()
                .filter(|n| **n == selected)
                .map(|_| { "(latest, recommended)" })
        )
        .spacing(5)
        .align_y(Alignment::Center),
    ]
    .spacing(5)
}

impl MenuInstallForge {
    pub fn view(&'_ self) -> Element<'_> {
        let main_block = widget::column!(
            widget::text("Installing Forge/NeoForge...").size(20),
            self.forge_progress.view()
        )
        .spacing(10);

        if self.is_java_getting_installed {
            widget::column!(main_block, self.java_progress.view())
        } else {
            main_block
        }
        .padding(20)
        .spacing(10)
        .into()
    }
}
