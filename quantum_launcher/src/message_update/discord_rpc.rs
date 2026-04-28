use filthy_rich::types::{Activity, ActivityType};
use iced::{Task, futures::executor::block_on};

use ql_core::{Instance, err, json::VersionDetails, pt};

use crate::{
    config::discord_rpc::PresenceStatusDisplayType,
    state::{Launcher, Message, RpcMessage},
};

impl Launcher {
    pub fn update_rpc(&mut self, msg: RpcMessage) -> Task<Message> {
        let rpc = self.config.discord_rpc.get_or_insert_default();

        match msg {
            RpcMessage::RunStarted(c) => {
                if let Some(c) = c {
                    if !self.config.c_rpc_enabled() {
                        if let Err(err) = block_on(c.close()) {
                            err!(no_log, "{err}");
                        }
                        return Task::none();
                    }
                    self.discord_ipc_client = Some(c);
                    return self.set_custom_discord_presence();
                } else {
                    pt!(
                        no_log,
                        "Rich presence couldn't be set as client wasn't found post-run."
                    )
                }
            }
            RpcMessage::Toggle(enable) => {
                rpc.enable = enable;

                if enable {
                    // Start on enable
                    return self.start_discord_ipc_run();
                }

                // On disable
                block_on(async {
                    if let Some(c) = &self.discord_ipc_client {
                        let _ = c.close().await;
                    }
                });

                self.is_presence_running
                    .store(false, std::sync::atomic::Ordering::Relaxed);
                self.discord_ipc_client = None;
            }
            RpcMessage::DefaultChanged(op) => {
                rpc.basic.apply(op);
            }
            RpcMessage::SetName(name) => rpc.name = (!name.is_empty()).then_some(name),
            RpcMessage::TogglePresenceOnGameEvent(t) => {
                rpc.update_on_game_open = t;
            }
            RpcMessage::ToggleCompeting(t) => {
                rpc.competing = t;
            }
            RpcMessage::GameOpen(op) => {
                rpc.on_gameopen.apply(op);
            }
            RpcMessage::GameExit(op) => {
                rpc.on_gameexit.apply(op);
            }
            RpcMessage::SetPresenceNow => return self.set_custom_discord_presence(),
            RpcMessage::ResetPresence => {
                self.config.reset_presence();
            }
            RpcMessage::StatusDisplayTypePicked(dt) => rpc.status_display_type = dt,
        }
        Task::none()
    }

    fn set_custom_discord_presence(&self) -> Task<Message> {
        let Some(c) = self.discord_ipc_client.clone() else {
            return Task::none();
        };
        let rpc_config = self.config.discord_rpc.clone().unwrap_or_default();

        if rpc_config.basic.top_text.is_none() && rpc_config.basic.bottom_text.is_none() {
            return Task::none();
        }

        let name = rpc_config.name.clone();
        let competing = rpc_config.competing;
        let details = rpc_config.basic.top_text.clone();
        let details_url = rpc_config.basic.top_text_url.clone();
        let state = rpc_config.basic.bottom_text.clone();
        let state_url = rpc_config.basic.bottom_text_url.clone();
        let sdt = rpc_config.status_display_type;

        Task::perform(
            async move {
                _ = c
                    .set_activity(bake_activity(
                        name,
                        sdt,
                        competing,
                        details,
                        details_url,
                        state,
                        state_url,
                    ))
                    .await;
            },
            |_| Message::Nothing,
        )
    }

    pub fn rpc_game_update(&mut self, selected_instance: Instance, exited: bool) -> Task<Message> {
        if !self.config.c_rpc_enabled() {
            return Task::none();
        }
        let rpc_config = self.config.discord_rpc.get_or_insert_default();
        let info = if exited {
            &rpc_config.on_gameexit
        } else {
            &rpc_config.on_gameopen
        };

        if info.top_text.is_none() && info.bottom_text.is_none() {
            return Task::none();
        }

        let name = rpc_config.name.clone();
        let competing = rpc_config.competing;
        let details = info.top_text.clone();
        let details_url = info.top_text_url.clone();
        let state = info.bottom_text.clone();
        let state_url = info.bottom_text_url.clone();
        let sdt = rpc_config.status_display_type;

        let client = self.discord_ipc_client.clone();

        Task::perform(
            async move {
                if let Ok(version_details) = VersionDetails::load(&selected_instance).await {
                    if let Some(c) = client {
                        let instance = selected_instance.get_name();
                        let minecraft_vers = version_details.get_id();

                        _ = c
                            .set_activity(bake_activity(
                                name,
                                sdt,
                                competing,
                                details.map(|f| f.substitute(instance, minecraft_vers)),
                                details_url,
                                state.map(|f| f.substitute(instance, minecraft_vers)),
                                state_url,
                            ))
                            .await;
                    }
                }
            },
            |()| Message::Nothing,
        )
    }
}

trait StringPresenceExt {
    fn substitute(&self, instance: &str, minecraft_vers: &str) -> String;
}

impl StringPresenceExt for str {
    fn substitute(&self, instance: &str, minecraft_vers: &str) -> String {
        self.replace("${version}", minecraft_vers)
            .replace("${instance}", instance)
    }
}

/// Returns a fully-built [`filthy_rich::types::Activity`] object given the details and state.
pub fn bake_activity(
    name: Option<String>,
    sdt: PresenceStatusDisplayType,
    competing: bool,
    details: Option<String>,
    details_url: Option<String>,
    state: Option<String>,
    state_url: Option<String>,
) -> Activity {
    let mut activity = Activity::new()
        .activity_type(if competing {
            ActivityType::Competing
        } else {
            ActivityType::Playing
        })
        .status_display_type(match sdt {
            PresenceStatusDisplayType::Name => filthy_rich::types::StatusDisplayType::Name,
            PresenceStatusDisplayType::Details => filthy_rich::types::StatusDisplayType::Details,
            PresenceStatusDisplayType::State => filthy_rich::types::StatusDisplayType::State,
        });

    if let Some(n) = name {
        activity = activity.name(n)
    }
    if let Some(d) = details {
        activity = activity.details(d);

        if let Some(d_u) = details_url {
            activity = activity.details_url(d_u);
        }
    }
    if let Some(s) = state {
        activity = activity.state(s);

        if let Some(s_u) = state_url {
            activity = activity.state_url(s_u);
        }
    }

    activity.build()
}
