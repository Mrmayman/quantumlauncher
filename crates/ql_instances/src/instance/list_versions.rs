use ql_core::{JsonDownloadError, ListEntry, ListEntryKind, json::Manifest};

/// Returns a list of every downloadable version of Minecraft.
/// Sources the list from multiple places (see [`Manifest`]).
///
/// # Errors
/// If [`Manifest`] couldn't be downloaded or parsed into JSON
pub async fn list_versions() -> Result<(Vec<ListEntry>, String), JsonDownloadError> {
    let manifest = Manifest::download().await?;
    let latest = manifest
        .get_latest_release()
        .or_else(|| manifest.versions.first())
        .map(|version| version.id.clone())
        .unwrap_or_default();

    Ok((
        manifest
            .versions
            .into_iter()
            .map(|n| ListEntry {
                kind: ListEntryKind::calculate(&n.id, &n.r#type),
                supports_server: n.supports_server(),
                name: n.id,
            })
            .collect(),
        latest,
    ))
}
