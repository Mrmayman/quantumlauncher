use ezshortcut::Shortcut;
use iced::Task;

use crate::state::{Launcher, MenuShortcut, Message, ShortcutMessage, State};

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
        }
        Task::none()
    }
}
