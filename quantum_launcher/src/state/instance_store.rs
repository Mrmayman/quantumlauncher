use std::{collections::HashMap, sync::mpsc::Sender};

use iced::{Task, futures::executor::block_on};
use ql_core::{Instance, InstanceKind, LaunchedProcess, OS_NAME, err, read_log::LogLine};
use tokio::io::AsyncWriteExt;

use crate::state::{FsWatcher, GameProcess, InstanceLog, LogState, Message, State, get_entries};

#[derive(Default)]
pub struct InstanceStore {
    client_list: Option<Vec<String>>,
    server_list: Option<Vec<String>>,
    client_watcher: Option<FsWatcher>,
    server_watcher: Option<FsWatcher>,

    processes: HashMap<Instance, GameProcess>,
    logs: HashMap<Instance, InstanceLog>,
}

impl InstanceStore {
    pub fn tick(
        &mut self,
        tasks: &mut Vec<Task<Message>>,
        state: &mut State,
        selected_instance: Option<&Instance>,
    ) {
        let mut watch_reload = |watcher: Option<&FsWatcher>, kind| {
            if watcher.is_some_and(FsWatcher::has_changed) {
                tasks.push(Task::perform(get_entries(kind), Message::CoreListLoaded));
            }
        };
        watch_reload(self.client_watcher.as_ref(), InstanceKind::Client);
        watch_reload(self.server_watcher.as_ref(), InstanceKind::Server);

        for (instance, process) in &mut self.processes {
            let log_state = if let State::Launch(menu) = state {
                &mut menu.log_state
            } else {
                &mut None
            };
            Self::read_game_logs(
                process,
                instance,
                &mut self.logs,
                log_state,
                selected_instance,
            );
        }
    }

    pub fn safe_to_quit(&self) -> bool {
        self.processes.is_empty()
    }

    pub fn server_cmd_edit(&mut self, server: &Instance, c: String) {
        if let Some(log) = self.logs.get_mut(server) {
            log.command = c;
        }
    }

    pub fn server_cmd_submit(&mut self, server: &Instance) {
        if let Some(log) = self.logs.get_mut(server)
            && let Some(p) = self.processes.get_mut(server)
            && let Some((stdin, _)) = &mut p.server_input
        {
            let log_cloned = format!("{}\n", log.command);
            let future = stdin.write_all(log_cloned.as_bytes());
            // Make the input command visible in the log
            log.log.push(format!("> {}", log.command));

            log.command.clear();
            _ = block_on(future);
        }
    }

    fn read_game_logs(
        process: &GameProcess,
        instance: &Instance,
        logs: &mut HashMap<Instance, InstanceLog>,
        log_state: &mut Option<LogState>,
        selected_instance: Option<&Instance>,
    ) {
        let update_ui = selected_instance.is_some_and(|n| n == instance);

        while let Some(message) = process.receiver.as_ref().and_then(|n| n.try_recv().ok()) {
            let message = message.to_string();

            logs.entry(instance.clone())
                .or_insert_with(|| {
                    let log_start = format!(
                        "[00:00:00] [launcher/INFO] {} (OS: {OS_NAME})\n",
                        if instance.is_server() {
                            "Starting Minecraft server"
                        } else {
                            "Launching Minecraft"
                        },
                    );

                    if update_ui {
                        *log_state = Some(LogState {
                            content: iced::widget::text_editor::Content::with_text(&log_start),
                        });
                    }
                    InstanceLog {
                        log: vec![log_start],
                        has_crashed: false,
                        command: String::new(),
                    }
                })
                .log
                .push(message.clone());

            if update_ui && let Some(state) = log_state {
                use iced::widget::text_editor::{Action, Edit, Motion};
                // TODO: preserve selection
                let message = message.replace('\t', "    ");
                let content = &mut state.content;
                content.perform(Action::Move(Motion::DocumentEnd));
                content.perform(Action::Edit(Edit::Paste(std::sync::Arc::new(message))));
            }
        }
    }

    pub fn list_loaded(&mut self, list: Vec<String>, kind: InstanceKind) {
        let (self_list, self_watcher) = match kind {
            InstanceKind::Client => (&mut self.client_list, &mut self.client_watcher),
            InstanceKind::Server => (&mut self.server_list, &mut self.server_watcher),
        };
        *self_list = Some(list);

        if self_watcher.is_none() {
            let dir = kind.get_root_directory();
            let watcher = match FsWatcher::new(dir) {
                Ok(n) => n,
                Err(err) => {
                    err!("Couldn't start dir watcher! {err}");
                    return;
                }
            };
            *self_watcher = Some(watcher);
        }
    }

    pub fn list_get(&self, kind: InstanceKind) -> Option<&[String]> {
        match kind {
            InstanceKind::Client => self.client_list.as_deref(),
            InstanceKind::Server => self.server_list.as_deref(),
        }
    }

    pub fn list_has_no_instances(&self) -> bool {
        match (
            self.list_get(InstanceKind::Client),
            self.list_get(InstanceKind::Server),
        ) {
            (None, None) => false,
            (Some(n), None) | (None, Some(n)) => n.is_empty(),
            (Some(n1), Some(n2)) => n1.is_empty() && n2.is_empty(),
        }
    }

    pub fn is_running(&self, instance: &Instance) -> bool {
        self.processes.contains_key(instance)
    }

    pub fn logs_get(&self, instance: &Instance) -> Option<&InstanceLog> {
        self.logs.get(instance)
    }

    pub fn logs_clear(&mut self, instance: &Instance) {
        self.logs.remove(instance);
    }

    pub fn logs_set_crashed(&mut self, instance: &Instance) {
        if let Some(log) = self.logs.get_mut(instance) {
            log.has_crashed = true;
        }
    }

    pub fn instance_launched(&mut self, child: LaunchedProcess) -> Sender<LogLine> {
        let (sender, receiver) = std::sync::mpsc::channel();
        let server_input = block_on(child.child.lock())
            .stdin
            .take()
            .map(|n| (n, false));
        self.processes.insert(
            child.instance.clone(),
            GameProcess {
                child,
                receiver: Some(receiver),
                server_input,
            },
        );
        sender
    }

    pub fn terminate_instance(&mut self, instance: Instance) -> Task<Message> {
        match instance.kind {
            InstanceKind::Client => {
                if let Some(process) = self.processes.remove(&instance) {
                    let mut child = block_on(process.child.child.lock());
                    _ = child.start_kill();
                }
            }
            InstanceKind::Server => {
                if let Some(GameProcess {
                    server_input: Some((stdin, has_issued_stop_command)),
                    child,
                    ..
                }) = self.processes.get_mut(&instance)
                {
                    *has_issued_stop_command = true;
                    if child.is_classic_server {
                        _ = block_on(child.child.lock()).start_kill();
                    } else {
                        let future = stdin.write_all("stop\n".as_bytes());
                        _ = block_on(future);
                    }
                }
            }
        }
        Task::none()
    }

    pub fn instance_voluntarily_exited(
        &mut self,
        instance: &Instance,
        log_state: &mut Option<LogState>,
        selected_instance: Option<&Instance>,
    ) {
        if let Some(process) = self.processes.remove(instance) {
            Self::read_game_logs(
                &process,
                instance,
                &mut self.logs,
                log_state,
                selected_instance,
            );
        }
    }
}
