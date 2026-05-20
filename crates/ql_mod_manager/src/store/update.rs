use std::path::PathBuf;
use std::sync::mpsc::Sender;

use chrono::DateTime;
use chrono::Local;
use ql_core::InstanceConfigJson;
use ql_core::{GenericProgress, Instance, do_jobs, err, info, json::VersionDetails};

use crate::store::{get_latest_version_date, toggle_mods};

use super::{ModError, ModId, ModIndex, delete_mods, download_mods_bulk};

#[derive(Debug, Clone)]
pub struct ChangelogFile {
    pub path: PathBuf,
    pub filename: String,
}

pub async fn apply_updates(
    selected_instance: Instance,
    updates: Vec<(ModId, String)>,
    progress: Option<Sender<GenericProgress>>,
    make_changelog: bool,
) -> Result<Option<ChangelogFile>, ModError> {
    let mod_index = ModIndex::load(&selected_instance).await?;

    let update_ids: Vec<ModId> = updates.iter().map(|(id, _)| id.clone()).collect();

    let disabled_mods: Vec<_> = update_ids
        .iter()
        .filter_map(|n| mod_index.mods.get_key_value(n))
        .filter(|n| !n.1.enabled)
        .map(|n| n.0.clone())
        .collect();

    let changelog_entries = if make_changelog {
        build_changelog_entries(&mod_index, &updates)
    } else {
        Vec::new()
    };

    // It's as simple as that!
    delete_mods(update_ids.clone(), selected_instance.clone()).await?;
    download_mods_bulk(update_ids, selected_instance.clone(), progress).await?;

    let mut changelog_file = None;
    if make_changelog && !changelog_entries.is_empty() {
        changelog_file = write_changelog(changelog_entries, selected_instance.clone()).await;
    }

    // Ensure disabled mods stay disabled
    toggle_mods(disabled_mods, selected_instance).await?;

    Ok(changelog_file)
}

async fn write_changelog(
    entries: Vec<String>,
    selected_instance: Instance,
) -> Option<ChangelogFile> {
    let titles = entries.join("\n");
    let now = Local::now();
    let filename = format!("changelog-{}.txt", now.format("%Y-%m-%d-%H-%M"));
    let path = selected_instance
        .get_instance_path()
        .join("changelogs")
        .join(&filename);

    let parent = path.parent()?;
    if let Err(err) = tokio::fs::create_dir_all(parent).await {
        err!("Failed to create changelog directory: {err}");
        return None;
    }
    if let Err(err) = tokio::fs::write(&path, &titles).await {
        err!("Failed to write changelog: {err}");
        return None;
    }

    Some(ChangelogFile { path, filename })
}

fn build_changelog_entries(mod_index: &ModIndex, updates: &[(ModId, String)]) -> Vec<String> {
    updates
        .iter()
        .map(|(mod_id, new_version)| {
            let (name, old_version) = match mod_index.mods.get(mod_id) {
                Some(mod_cfg) => (mod_cfg.name.clone(), mod_cfg.installed_version.clone()),
                None => (mod_id.get_internal_id(), String::new()),
            };

            let name = trim(&name);
            let old_version = trim(&old_version);
            let new_version = trim(new_version);

            format!("- {name}: {old_version} -> {new_version}")
        })
        .collect()
}

fn trim(value: &str) -> &str {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        "unknown"
    } else {
        trimmed
    }
}

pub async fn check_for_updates(instance: Instance) -> Result<Vec<(ModId, String)>, ModError> {
    let index = ModIndex::load(&instance).await?;
    let version_json = VersionDetails::load(&instance).await?;
    let config = InstanceConfigJson::read(&instance).await?;

    let loader = config.mod_type;

    info!(
        "Checking for mod updates (instance: {}, loader: {loader})",
        instance.get_name()
    );

    let version = version_json.get_id();

    let updated_mods: Result<Vec<Option<(ModId, String)>>, ModError> = do_jobs(
        index
            .mods
            .into_iter()
            .map(|(mod_id, installed_mod)| async move {
                let (download_version_time, download_version) =
                    get_latest_version_date(loader, &mod_id, version).await?;

                let installed_version_time =
                    DateTime::parse_from_rfc3339(&installed_mod.version_release_time)?;

                Ok((download_version_time > installed_version_time)
                    .then_some((mod_id, download_version)))
            }),
    )
    .await;
    let updated_mods: Vec<(ModId, String)> = updated_mods?.into_iter().flatten().collect();

    if updated_mods.is_empty() {
        info!("No mod updates found");
    } else {
        info!("Found mod updates");
    }

    Ok(updated_mods)
}
