use iced::{
    Alignment, Length,
    widget::{self, column},
};
use ql_core::Instance;
use ql_mod_manager::loaders::fabric::{
    self, BackendType, FabricVersionList, FabricVersionListItem,
};

use crate::menu_renderer::Column;
use crate::state::{InstallPaperMessage, MenuInstallPaper};
use crate::{
    icons,
    menu_renderer::{Element, back_button, button_with_icon},
    state::{
        InstallFabricMessage, InstallOptifineMessage, ManageModsMessage, MenuInstallFabric,
        MenuInstallOptifine, Message,
    },
    stylesheet::styles::LauncherTheme,
};

impl MenuInstallOptifine {
    pub fn view(&'_ self) -> Element<'_> {
        match self {
            MenuInstallOptifine::InstallingB173 => {
                column![widget::text("Installing OptiFine for Beta 1.7.3...").size(20)]
                    .padding(16)
                    .into()
            }
            MenuInstallOptifine::Installing(bar) => {
                column![widget::text("Installing OptiFine").size(20), bar.view()]
                    .padding(16)
                    .spacing(10)
                    .into()
            }
            MenuInstallOptifine::Choosing {
                delete_installer,
                drag_and_drop_hovered,
                ..
            } => {
                let menu = self
                    .install_optifine_screen(*delete_installer)
                    .padding(10)
                    .spacing(10);
                widget::stack!(
                    menu,
                    if *drag_and_drop_hovered {
                        Some(widget::center(widget::button(
                            widget::text("Drag and drop the OptiFine installer").size(20),
                        )))
                    } else {
                        None
                    }
                )
                .into()
            }
        }
    }

    fn install_optifine_screen<'a>(
        &self,
        delete_installer: bool,
    ) -> widget::Column<'a, Message, LauncherTheme, iced::Renderer> {
        column![
            back_button().on_press(ManageModsMessage::Open.into()),
            widget::container(
                column![
                    widget::text("Install OptiFine").size(20),
                    "Step 1: Open the OptiFine download page and download the installer.",
                    "WARNING: Make sure to download the correct version.",
                    widget::button("Open download page")
                        .on_press(Message::CoreOpenLink(self.get_url().to_owned()))
                ]
                .padding(10)
                .spacing(10)
            ),
            widget::container(
                column![
                    "Step 2: Select the installer file",
                    widget::checkbox(delete_installer)
                        .label("Delete installer after use")
                        .on_toggle(|t| InstallOptifineMessage::DeleteInstallerToggle(t).into()),
                    widget::button("Select File")
                        .on_press(InstallOptifineMessage::SelectInstallerStart.into())
                ]
                .padding(10)
                .spacing(10)
            )
        ]
        .width(Length::Fill)
        .height(Length::Fill)
    }
}

impl MenuInstallFabric {
    pub fn view(&'_ self, selected_instance: &Instance, tick_timer: usize) -> Element<'_> {
        match self {
            MenuInstallFabric::Loading { is_quilt, .. } => {
                let loader_name = if *is_quilt { "Quilt" } else { "Fabric" };
                let dots = ".".repeat((tick_timer % 3) + 1);

                column![
                    back_button().on_press(ManageModsMessage::Open.into()),
                    widget::text!("Loading {loader_name} version list{dots}").size(20)
                ]
            }
            MenuInstallFabric::Loaded {
                progress: Some(progress),
                backend,
                ..
            } => {
                column![
                    widget::text!("Installing {backend}...").size(20),
                    progress.view(),
                ]
            }
            MenuInstallFabric::Unsupported(is_quilt) => {
                column![
                    back_button().on_press(ManageModsMessage::Open.into()),
                    widget::text!(
                        "{} is unsupported for this Minecraft version.",
                        if *is_quilt { "Quilt" } else { "Fabric" }
                    )
                ]
            }
            MenuInstallFabric::Loaded {
                fabric_versions: FabricVersionList::Unsupported,
                backend,
                ..
            } => {
                column![
                    back_button().on_press(ManageModsMessage::Open.into()),
                    widget::text!("{backend} is unsupported for this Minecraft version.")
                ]
            }
            MenuInstallFabric::Loaded {
                backend,
                fabric_version,
                fabric_versions,
                ..
            } => install_fabric_main(selected_instance, backend, fabric_version, fabric_versions),
        }
        .padding(10)
        .spacing(10)
        .into()
    }
}

fn install_fabric_main<'a>(
    selected_instance: &Instance,
    backend: &'a BackendType,
    fabric_version: &'a str,
    fabric_versions: &'a FabricVersionList,
) -> widget::Column<'a, Message, LauncherTheme> {
    let picker = match fabric_versions {
        FabricVersionList::Quilt(l)
        | FabricVersionList::Fabric(l)
        | FabricVersionList::LegacyFabric(l)
        | FabricVersionList::OrnitheMCQuilt(l)
        | FabricVersionList::OrnitheMCFabric(l) => version_list(l, fabric_version),

        FabricVersionList::Beta173 {
            ornithe_mc,
            babric,
            cursed_legacy,
        } => {
            let list = match backend {
                BackendType::OrnitheMCFabric => ornithe_mc,
                BackendType::Babric => babric,
                BackendType::CursedLegacy => cursed_legacy,
                _ => unreachable!(),
            };

            widget::column![
                "Pick an implementation of Fabric for beta 1.7.3:",
                widget::pick_list(
                    [
                        BackendType::Babric,
                        BackendType::OrnitheMCFabric,
                        BackendType::CursedLegacy
                    ],
                    Some(backend),
                    |b| InstallFabricMessage::ChangeBackend(b).into()
                ),
                version_list(list, fabric_version),
            ]
            .spacing(5)
        }
        FabricVersionList::Both {
            legacy_fabric,
            ornithe_mc,
        } => {
            let list = match backend {
                BackendType::LegacyFabric => legacy_fabric,
                BackendType::OrnitheMCFabric => ornithe_mc,
                _ => unreachable!(),
            };

            widget::column![
                "Pick an implementation of Fabric:",
                widget::pick_list(
                    [BackendType::LegacyFabric, BackendType::OrnitheMCFabric,],
                    Some(backend),
                    |b| InstallFabricMessage::ChangeBackend(b).into()
                ),
                version_list(list, fabric_version),
            ]
            .spacing(5)
        }

        FabricVersionList::Unsupported => unreachable!(),
    };

    widget::column![
        back_button().on_press(ManageModsMessage::Open.into()),
        widget::text!("Install {backend} for \"{}\"", selected_instance.get_name()).size(20),
        picker,
        button_with_icon(icons::download(), "Install", 16)
            .on_press(InstallFabricMessage::ButtonClicked.into()),
    ]
}

fn version_list<'a>(list: &'a [FabricVersionListItem], selected: &'a str) -> Column<'a> {
    let selected = FabricVersionListItem {
        loader: fabric::FabricVersion {
            version: selected.to_owned(),
        },
    };
    column![
        widget::text("Version:"),
        widget::row![
            widget::pick_list(list, Some(selected.clone()), |n| {
                InstallFabricMessage::VersionSelected(n.loader.version).into()
            }),
            list.first()
                .filter(|n| **n == selected)
                .map(|_| { "(latest, recommended)" })
        ]
        .spacing(5)
        .align_y(Alignment::Center),
    ]
    .spacing(5)
}

impl MenuInstallPaper {
    pub fn view(&'_ self, tick_timer: usize) -> Element<'_> {
        let dots = ".".repeat((tick_timer % 3) + 1);
        match self {
            MenuInstallPaper::Loading { .. } => column![
                back_button().on_press(ManageModsMessage::Open.into()),
                widget::text!("Loading{dots}").size(20),
            ]
            .padding(10)
            .spacing(10)
            .into(),
            MenuInstallPaper::Loaded { version, versions } => column![
                back_button().on_press(ManageModsMessage::Open.into()),
                widget::text!("Select Version").size(20),
                widget::row![
                    widget::pick_list(versions.clone(), Some(version), |v| {
                        Message::InstallPaper(InstallPaperMessage::VersionSelected(v))
                    }),
                    versions
                        .first()
                        .is_some_and(|n| n == version)
                        .then_some("(latest, recommended)")
                ]
                .align_y(Alignment::Center),
                button_with_icon(icons::download(), "Install", 16)
                    .on_press(Message::InstallPaper(InstallPaperMessage::ButtonClicked)),
            ]
            .padding(10)
            .spacing(10)
            .into(),
            MenuInstallPaper::Installing => {
                column![widget::text!("Installing Paper{dots}").size(20)]
                    .padding(10)
                    .into()
            }
        }
    }
}
