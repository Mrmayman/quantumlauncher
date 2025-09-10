// Create tab: version search, filtering, and instance creation

use crate::tui::app::{App, VersionCategory};

impl App {
    pub fn start_version_search(&mut self) {
        if !self.version_search_active {
            self.version_search_active = true;
            self.version_search_query.clear();
            self.filtered_versions = self.available_versions.clone();
            self.selected_filtered_version = 0;
            self.status_message =
                "Search versions: type to filter, Backspace to edit, Esc to cancel".to_string();
        }
    }

    pub fn exit_version_search(&mut self) {
        if self.version_search_active {
            self.version_search_active = false;
            self.version_search_query.clear();
            self.filtered_versions.clear();
            self.selected_filtered_version = 0;
            self.status_message = "Exited version search".to_string();
        }
    }

    pub fn add_char_to_version_search(&mut self, c: char) {
        if self.version_search_active {
            self.version_search_query.push(c);
            self.update_filtered_versions();
        }
    }
    pub fn remove_char_from_version_search(&mut self) {
        if self.version_search_active {
            if self.version_search_query.pop().is_none() {
                self.exit_version_search();
            } else {
                self.update_filtered_versions();
            }
        }
    }

    pub(crate) fn update_filtered_versions(&mut self) {
        let query = self.version_search_query.to_lowercase();
        self.filtered_versions = self
            .available_versions
            .iter()
            .filter(|v| {
                let cat = Self::classify_version(&v.name);
                let type_ok = match cat {
                    VersionCategory::Release => self.filter_release,
                    VersionCategory::Snapshot => self.filter_snapshot,
                    VersionCategory::Beta => self.filter_beta,
                    VersionCategory::Alpha => self.filter_alpha,
                };
                if !type_ok {
                    return false;
                }
                if query.is_empty() {
                    true
                } else {
                    v.name.to_lowercase().contains(&query)
                }
            })
            .cloned()
            .collect();
        if self.selected_filtered_version >= self.filtered_versions.len() {
            self.selected_filtered_version = 0;
        }
    }

    pub fn classify_version(name: &str) -> VersionCategory {
        if ql_core::REGEX_SNAPSHOT.is_match(name) || name.contains("-pre") || name.contains("-rc") {
            return VersionCategory::Snapshot;
        }
        if name.starts_with('a') {
            return VersionCategory::Alpha;
        }
        if name.starts_with('b') || name.starts_with("inf-") {
            return VersionCategory::Beta;
        }
        VersionCategory::Release
    }

    pub fn toggle_filter_release(&mut self) {
        self.filter_release = !self.filter_release;
        self.update_filtered_versions();
    }
    pub fn toggle_filter_snapshot(&mut self) {
        self.filter_snapshot = !self.filter_snapshot;
        self.update_filtered_versions();
    }
    pub fn toggle_filter_beta(&mut self) {
        self.filter_beta = !self.filter_beta;
        self.update_filtered_versions();
    }
    pub fn toggle_filter_alpha(&mut self) {
        self.filter_alpha = !self.filter_alpha;
        self.update_filtered_versions();
    }
    pub fn reset_all_filters(&mut self) {
        self.filter_release = true;
        self.filter_snapshot = true;
        self.filter_beta = true;
        self.filter_alpha = true;
        self.update_filtered_versions();
    }

    pub fn create_instance(&mut self) {
        if self.new_instance_name.is_empty()
            || self.available_versions.is_empty()
            || self.is_loading
        {
            return;
        }
        let instance_name = self.new_instance_name.clone();
        if self.filtered_versions.is_empty() {
            return;
        }
        let version = self.filtered_versions[self.selected_filtered_version].clone();
        let download_assets = self.download_assets;
        self.is_loading = true;
        self.status_message = format!(
            "Creating instance '{}'... This may take a while",
            instance_name
        );
        if let Some(sender) = &self.auth_sender {
            let _ = sender.send(crate::tui::AuthEvent::InstanceCreateStarted(
                instance_name.clone(),
            ));
        }
        let auth_sender = self.auth_sender.clone();
        let instance_name_for_spawn = instance_name.clone();
        tokio::spawn(async move {
            let (progress_sender, progress_receiver) = std::sync::mpsc::channel();
            if let Some(sender) = auth_sender.clone() {
                let instance_name_for_progress = instance_name_for_spawn.clone();
                tokio::spawn(async move {
                    while let Ok(progress) = progress_receiver.try_recv() {
                        let message = match progress {
                            ql_core::DownloadProgress::DownloadingJsonManifest => {
                                "Downloading manifest...".to_string()
                            }
                            ql_core::DownloadProgress::DownloadingVersionJson => {
                                "Downloading version JSON...".to_string()
                            }
                            ql_core::DownloadProgress::DownloadingJar => {
                                "Downloading game jar...".to_string()
                            }
                            ql_core::DownloadProgress::DownloadingAssets { progress, out_of } => {
                                format!("Downloading assets ({}/{})", progress, out_of)
                            }
                            ql_core::DownloadProgress::DownloadingLibraries {
                                progress,
                                out_of,
                            } => {
                                format!("Downloading libraries ({}/{})", progress, out_of)
                            }
                            ql_core::DownloadProgress::DownloadingLoggingConfig => {
                                "Downloading logging config...".to_string()
                            }
                        };
                        let _ = sender.send(crate::tui::AuthEvent::InstanceCreateProgress {
                            instance_name: instance_name_for_progress.clone(),
                            message,
                        });
                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    }
                });
            }
            let result = ql_instances::create_instance(
                instance_name_for_spawn.clone(),
                version.clone(),
                Some(progress_sender),
                download_assets,
            )
            .await;
            if let Some(sender) = auth_sender {
                match result {
                    Ok(_) => {
                        let _ = sender.send(crate::tui::AuthEvent::InstanceCreateSuccess {
                            instance_name: instance_name_for_spawn.clone(),
                        });
                    }
                    Err(e) => {
                        let _ = sender.send(crate::tui::AuthEvent::InstanceCreateError {
                            instance_name: instance_name_for_spawn.clone(),
                            error: e.to_string(),
                        });
                    }
                }
            }
        });
        self.new_instance_name.clear();
        self.is_editing_name = false;
        self.download_assets = true;
    }

    // set_instance_name removed; use direct mutation where needed in input handlers
}
