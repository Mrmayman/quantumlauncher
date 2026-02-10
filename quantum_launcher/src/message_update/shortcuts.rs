use ezshortcut::Shortcut;
use iced::Task;
use ql_core::{info, IntoStringError};

use crate::state::{
    Launcher, MenuShortcut, Message, ShortcutMessage, State, NEW_ACCOUNT_NAME, OFFLINE_ACCOUNT_NAME,
};

macro_rules! iflet {
    ($m:ident, $s:expr, $b:block) => {{
        if let State::CreateShortcut($m) = &mut $s {
            $b
        }
    }};
}

impl Launcher {
    pub fn update_shortcut(&mut self, msg: ShortcutMessage) -> Task<Message> {
        match msg {
            ShortcutMessage::Open => {
                self.state = State::CreateShortcut(MenuShortcut {
                    shortcut: Shortcut {
                        name: self.instance().get_name().to_owned(),
                        description: "".to_owned(),
                        exec: String::new(),
                        icon: None,
                    },
                    add_to_menu: true,
                    add_to_desktop: false,
                    account: self.account_selected.clone(),
                    account_offline: self.config.username.clone(),
                })
            }
            ShortcutMessage::ToggleAddToMenu(t) => iflet!(menu, self.state, {
                if t || menu.add_to_desktop {
                    menu.add_to_menu = t;
                }
            }),
            ShortcutMessage::ToggleAddToDesktop(t) => iflet!(menu, self.state, {
                if t || menu.add_to_menu {
                    menu.add_to_desktop = t;
                }
            }),
            ShortcutMessage::EditName(name) => iflet!(menu, self.state, {
                menu.shortcut.name = name;
            }),
            ShortcutMessage::EditDescription(desc) => iflet!(menu, self.state, {
                menu.shortcut.description = desc;
            }),
            ShortcutMessage::AccountSelected(acc) => iflet!(menu, self.state, {
                if acc == NEW_ACCOUNT_NAME {
                    self.state = State::AccountLogin;
                } else {
                    menu.account = acc;
                }
            }),
            ShortcutMessage::AccountOffline(acc) => iflet!(menu, self.state, {
                menu.account_offline = acc;
            }),

            ShortcutMessage::SaveCustom => iflet!(menu, self.state, {
                return Task::perform(
                    rfd::AsyncFileDialog::new()
                        .add_filter("Shortcut", &[ezshortcut::EXTENSION])
                        .set_file_name(menu.shortcut.get_filename())
                        .set_title("Save shortcut to...")
                        .save_file(),
                    |f| {
                        if let Some(f) = f {
                            Message::Shortcut(ShortcutMessage::SaveCustomPicked(
                                f.path().to_owned(),
                            ))
                        } else {
                            Message::Nothing
                        }
                    },
                );
            }),
            ShortcutMessage::SaveCustomPicked(f) => iflet!(menu, self.state, {
                let mut shortcut = menu.shortcut.clone();
                let instance = self.selected_instance.as_ref().unwrap();

                let exec_path = match std::env::current_exe() {
                    Ok(n) => n,
                    Err(err) => {
                        self.set_error(format!("while getting path to current exe:\n{err}"));
                        return Task::none();
                    }
                };
                let launch = format!(
                    "{exe} {server}launch {name} {username}{login} --show-progress",
                    exe = exec_path.to_string_lossy(),
                    server = if instance.is_server() {
                        "--enable-server-manager -s "
                    } else {
                        ""
                    },
                    name = serde_json::to_string(&instance.get_name())
                        .unwrap_or_else(|_| instance.get_name().to_owned()),
                    username = if menu.account == OFFLINE_ACCOUNT_NAME {
                        &menu.account_offline
                    } else {
                        &menu.account
                    },
                    login = if menu.account == OFFLINE_ACCOUNT_NAME {
                        ""
                    } else {
                        " -u"
                    }
                );
                shortcut.exec = launch;

                return Task::perform(async move { shortcut.generate(&f).await }, |n| {
                    Message::Shortcut(ShortcutMessage::Done(n.strerr()))
                });
            }),
            ShortcutMessage::Done(r) => match r {
                Ok(()) => {
                    info!("Created instance shortcut");
                    // TODO: keep track of created shortcuts
                }
                Err(err) => self.set_error(err),
            },
        }
        Task::none()
    }
}
