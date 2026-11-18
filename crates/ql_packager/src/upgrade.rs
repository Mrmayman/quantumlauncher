use std::collections::HashSet;

use ql_core::{
    Instance, InstanceConfigJson, InstanceKind, IntoIoError, ListEntry, Loader, err,
    file_utils::copy_dir_recursive_ext, json::VersionDetails,
};
use ql_mod_manager::{
    loaders::install_specified_loader,
    store::{CurseforgeNotAllowed, ModIndex, QueryType, get_locally_installed_mods},
};

use crate::{EXCEPTIONS, InstancePackageError, SOFT_EXCEPTIONS};

// TODO: progress bar Sender<>

pub async fn upgrade(
    instance: Instance,
) -> Result<HashSet<CurseforgeNotAllowed>, InstancePackageError> {
    let details_old = VersionDetails::load(&instance).await?;
    let config_old = InstanceConfigJson::read(&instance).await?;
    let index_old = ModIndex::load(&instance).await?;

    let is_legacy_texturepacks = details_old.is_legacy_texturepacks();

    let old_instance = rename_to_old(&instance).await?;
    let old_instance_dir = old_instance.get_instance_path();
    let new_instance_dir = instance.get_instance_path();

    let name = instance.name.to_string();
    let version = ListEntry::new(details_old.get_id().to_owned());

    match instance.kind {
        InstanceKind::Server => ql_servers::create_server(name, version, None).await?,
        InstanceKind::Client => ql_instances::create_instance(name, version, None, true).await?,
    };

    let loader = config_old.mod_type;
    if let Loader::Fabric | Loader::Quilt = loader {
        todo!()
    } else if let Loader::OptiFine = loader {
        todo!()
    } else {
        let version = config_old
            .mod_type_info
            .as_ref()
            .and_then(|n| n.version.clone());

        install_specified_loader(instance.clone(), loader, None, version)
            .await
            .map_err(InstancePackageError::Loader)?;
    }

    let not_allowed = ql_mod_manager::store::download_mods_bulk(
        index_old.mods.keys().cloned().collect(),
        instance.clone(),
        None,
    )
    .await?;

    for project_type in QueryType::INDEX_SUPPORTED {
        let blacklist = index_old.get_downloaded_files(*project_type);
        let local_mods =
            get_locally_installed_mods(instance.get_dot_minecraft_path(), blacklist, *project_type)
                .await;

        let dirname = project_type.get_dirname(is_legacy_texturepacks);

        let new_parent = new_instance_dir.join(dirname);
        tokio::fs::create_dir_all(&new_parent)
            .await
            .path(&new_parent)?;

        for m in local_mods {
            let old_path = old_instance_dir.join(dirname).join(&*m.0);
            let new_path = new_parent.join(&*m.0);

            if let Err(err) = tokio::fs::copy(&old_path, &new_path).await.path(&old_path) {
                err!("While copying data over: {err}");
            }
        }
    }

    // Also transfer .minecraft files
    let old_dotmc = old_instance.get_dot_minecraft_path();
    let exceptions: Vec<_> = SOFT_EXCEPTIONS
        .iter()
        .chain(EXCEPTIONS)
        .map(|n| old_dotmc.join(n))
        .collect();
    copy_dir_recursive_ext(&old_dotmc, &instance.get_dot_minecraft_path(), &exceptions).await?;

    todo!()
}

async fn rename_to_old(instance: &Instance) -> Result<Instance, InstancePackageError> {
    todo!()
}
