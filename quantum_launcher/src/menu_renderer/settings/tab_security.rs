use iced::{Alignment, widget};
use ql_auth::{TokenStorageMethod, encrypted_store};
use ql_core::LAUNCHER_DIR;

use crate::{
    config::LauncherConfig,
    icons,
    menu_renderer::{Column, button_with_icon, checkered_list, tsubtitle},
    state::{Message, TokenStoreMessage},
};

pub(super) fn view<'a>(config: &'a LauncherConfig, keyring_available: bool) -> Column<'a> {
    let current_method = config.c_token_storage();
    let file_exists = encrypted_store::file_exists();
    let is_unlocked = encrypted_store::is_unlocked();

    let method_label = widget::text("Account Token Storage:").size(14);

    let keyring_btn = widget::button(if current_method == TokenStorageMethod::Keyring {
        "* System Keyring"
    } else {
        "  System Keyring"
    })
    .on_press_maybe(
        (current_method != TokenStorageMethod::Keyring)
            .then_some(TokenStoreMessage::TokenStorageChanged(TokenStorageMethod::Keyring).into()),
    );

    let encrypted_btn = widget::button(if current_method == TokenStorageMethod::EncryptedFile {
        "* Encrypted File"
    } else {
        "  Encrypted File"
    })
    .on_press_maybe(
        (current_method != TokenStorageMethod::EncryptedFile).then_some(
            TokenStoreMessage::TokenStorageChanged(TokenStorageMethod::EncryptedFile).into(),
        ),
    );

    let mut security = widget::column![
        method_label,
        widget::row![keyring_btn, encrypted_btn].spacing(8),
    ]
    .spacing(8);

    if current_method == TokenStorageMethod::Keyring && !keyring_available {
        security = security.push(
            widget::text("SYSTEM keyring is unavailable")
                .size(12)
                .style(tsubtitle)
                .color(iced::Color::from_rgb(0.9, 0.3, 0.3)),
        );
    }

    security = security
        .push(
            widget::text(
                "Encrypted File stores tokens in an AES-256-GCM encrypted file\nthat can be moved to other machines.",
            )
            .size(12),
        )
        .push(widget::horizontal_rule(1));

    if current_method == TokenStorageMethod::EncryptedFile || file_exists {
        if file_exists {
            let status_text = if is_unlocked {
                "Status: Unlocked"
            } else {
                "Status: Locked"
            };
            security = security.push(widget::text(status_text).size(12));

            if !is_unlocked {
                security = security.push(
                    widget::button("Unlock Store")
                        .on_press(TokenStoreMessage::UnlockEncryptedStore.into()),
                );
            }

            security = security.push(
                widget::row![
                    button_with_icon(icons::bin_s(12), "Delete Store", 12)
                        .padding([5, 10])
                        .on_press(TokenStoreMessage::DeleteEncryptedStore.into()),
                    widget::text("Deletes the encrypted file and removes all associated accounts.")
                        .size(12),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            );
        } else {
            security = security.push(
                widget::column![
                    widget::text("No encrypted store exists yet.").size(12),
                    widget::button("Create Encrypted Store")
                        .on_press(TokenStoreMessage::SetupEncryptedStore.into()),
                ]
                .spacing(6),
            );
        }

        security = security.push(
            button_with_icon(icons::folder_s(12), "Open Launcher Folder", 12)
                .padding([5, 10])
                .on_press(Message::CoreOpenPath(LAUNCHER_DIR.clone())),
        );
    }

    checkered_list([
        widget::column![widget::text("Security").size(20)],
        security,
    ])
}
