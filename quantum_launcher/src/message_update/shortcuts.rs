use ezshortcut::Shortcut;
use iced::Task;
use ql_core::{info, IntoStringError, LAUNCHER_DIR};

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
    pub fn update_shortcut(&mut self, msg: ShortcutMessage) -> Result<Task<Message>, String> {
        match msg {
            ShortcutMessage::Open => {
                self.shortcut_open();
            }
            ShortcutMessage::OpenFolder => {
                if let Some(dir) = ezshortcut::get_menu_path() {
                    _ = open::that(&dir);
                }
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
                return Ok(Task::perform(
                    rfd::AsyncFileDialog::new()
                        .add_filter("Shortcut", &[ezshortcut::EXTENSION_S])
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
                ));
            }),
            ShortcutMessage::SaveCustomPicked(path) => {
                let shortcut = self.shortcut_prepare()?;
                return Ok(Task::perform(
                    async move { shortcut.generate(&path).await },
                    |n| Message::Shortcut(ShortcutMessage::Done(n.strerr())),
                ));
            }
            ShortcutMessage::SaveMenu => {
                let shortcut = self.shortcut_prepare()?;
                if let State::CreateShortcut(menu) = &self.state {
                    return Ok(Task::batch([
                        if menu.add_to_desktop {
                            shortcut_desktop(&shortcut)?
                        } else {
                            Task::none()
                        },
                        if menu.add_to_menu {
                            shortcut_menu(shortcut)
                        } else {
                            Task::none()
                        },
                    ]));
                }
            }
            ShortcutMessage::Done(result) => {
                result?;
                info!("Created shortcut");
                // TODO: keep track of created shortcuts
            }
        }
        Ok(Task::none())
    }

    fn shortcut_open(&mut self) {
        self.state = State::CreateShortcut(MenuShortcut {
            shortcut: Shortcut {
                name: self.instance().get_name().to_owned(),
                description: "".to_owned(),
                exec: String::new(),
                exec_args: vec![],
                icon: None,
            },
            add_to_menu: true,
            add_to_desktop: false,
            account: self.account_selected.clone(),
            account_offline: self.config.username.clone(),
        })
    }

    pub fn shortcut_prepare(&mut self) -> Result<Shortcut, String> {
        let State::CreateShortcut(menu) = &self.state else {
            self.shortcut_open();
            return self.shortcut_prepare();
        };
        let mut shortcut = menu.shortcut.clone();
        let instance = self.selected_instance.as_ref().unwrap();

        let exec_path = std::env::current_exe()
            .map_err(|n| format!("while getting path to current exe:\n{n}"))?;

        shortcut.exec = exec_path.to_string_lossy().to_string();

        // Environment setup
        if instance.is_server() {
            shortcut
                .exec_args
                .push("--enable-server-manager".to_owned());
            shortcut.exec_args.push("-s".to_owned());
        }
        if let Some(n) = dirs::data_dir().map(|d| d.join("QuantumLauncher")) {
            if *LAUNCHER_DIR != n {
                shortcut.exec_args.push("--dir".to_owned());
                shortcut
                    .exec_args
                    .push(LAUNCHER_DIR.to_string_lossy().to_string());
            }
        }

        // Launch command
        shortcut
            .exec_args
            .extend(["launch".to_owned(), instance.get_name().to_owned()]);

        // Account setup
        if menu.account == OFFLINE_ACCOUNT_NAME {
            shortcut.exec_args.push(menu.account_offline.clone());
        } else {
            shortcut.exec_args.push(menu.account.clone());
            shortcut.exec_args.push("-u".to_owned());
        }

        shortcut.exec_args.push("--show-progress".to_owned());

        Ok(shortcut)
    }
}

fn shortcut_menu(shortcut: Shortcut) -> Task<Message> {
    Task::perform(
        async move { shortcut.generate_to_applications().await },
        |n| Message::Shortcut(ShortcutMessage::Done(n.strerr())),
    )
}

fn shortcut_desktop(shortcut: &Shortcut) -> Result<Task<Message>, String> {
    let desktop =
        ezshortcut::get_desktop_dir().ok_or_else(|| "Couldn't access Desktop folder".to_owned())?;
    let s = shortcut.clone();
    Ok(Task::perform(
        async move { s.generate(&desktop).await },
        |n| Message::Shortcut(ShortcutMessage::Done(n.strerr())),
    ))
}
