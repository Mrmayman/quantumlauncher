use chrono::DateTime;
use ql_core::{
    file_utils, info,
    json::{instance_config::ModTypeInfo, VersionDetails},
    no_window, pipe_progress_ext, pt, GenericProgress, InstanceSelection, IntoIoError,
    IntoJsonError, IoError, Loader, CLASSPATH_SEPARATOR, REGEX_SNAPSHOT,
};
use ql_java_handler::{get_java_binary, JavaVersion, JAVA};
use serde::Deserialize;
use sipper::Sender;
use std::{fmt::Write, io::Cursor, path::Path};
use tokio::{fs, process::Command};

use crate::loaders::change_instance_type;

use super::forge::{ForgeInstallError, ForgeProgress};

const NEOFORGE_VERSIONS_URL: &str =
    "https://maven.neoforged.net/api/maven/versions/releases/net/neoforged/neoforge";

const INSTALLER_NAME: &str = "installer.jar";

#[derive(Deserialize)]
struct NeoforgeVersions {
    versions: Vec<String>,
}

pub async fn install(
    neoforge_version: Option<String>,
    instance: InstanceSelection,
    mut f_progress: Option<Sender<ForgeProgress>>,
) -> Result<(), ForgeInstallError> {
    info!("Installing NeoForge");
    let (neoforge_version, json) =
        get_version_and_json(neoforge_version, &instance, f_progress.as_mut()).await?;
    let installer_bytes = get_installer(f_progress.as_mut(), &neoforge_version).await?;

    let instance_dir = instance.get_instance_path();
    let neoforge_dir = instance_dir.join("forge");
    fs::create_dir_all(&neoforge_dir)
        .await
        .path(&neoforge_dir)?;
    if !instance.is_server() {
        create_required_jsons(&neoforge_dir).await?;
    }

    let installer_path = neoforge_dir.join(INSTALLER_NAME);
    fs::write(&installer_path, &installer_bytes)
        .await
        .path(&installer_path)?;

    run_installer(&neoforge_dir, f_progress.clone(), instance.is_server()).await?;

    if instance.is_server() {
        fs::remove_dir_all(&neoforge_dir).await.path(neoforge_dir)?;
        delete(&instance_dir, "installer.jar.log").await?;
        delete(&instance_dir, "run.bat").await?;
        delete(&instance_dir, "run.sh").await?;
        delete(&instance_dir, "user_jvm_args.txt").await?;
    } else {
        download_libraries(f_progress.clone(), &json, &installer_bytes, &neoforge_dir).await?;
        delete(&neoforge_dir, "launcher_profiles.json").await?;
        delete(&neoforge_dir, "launcher_profiles_microsoft_store.json").await?;
    }

    pt!("Finished");
    change_instance_type(
        &instance_dir,
        Loader::Neoforge,
        Some(ModTypeInfo {
            version: Some(neoforge_version),
            backend_implementation: None,
            optifine_jar: None,
        }),
    )
    .await?;

    Ok(())
}

async fn download_libraries(
    mut f_progress: Option<Sender<ForgeProgress>>,
    json: &VersionDetails,
    installer_bytes: &[u8],
    neoforge_dir: &Path,
) -> Result<(), ForgeInstallError> {
    let jar_version_json = get_version_json(installer_bytes, neoforge_dir, json).await?;

    let libraries_path = neoforge_dir.join("libraries");
    fs::create_dir_all(&libraries_path)
        .await
        .path(&libraries_path)?;

    let mut classpath = String::new();
    let mut clean_classpath = String::new();

    let len = jar_version_json.libraries.len();
    for (i, library) in jar_version_json
        .libraries
        .iter()
        .filter(|n| n.clientreq.unwrap_or(true))
        .enumerate()
    {
        info!("Downloading library {i}/{len}: {}", library.name);

        if let Some(progress) = &mut f_progress {
            progress
                .send(ForgeProgress::P6DownloadingLibrary {
                    num: i,
                    out_of: len,
                })
                .await;
        }
        let parts: Vec<&str> = library.name.split(':').collect();

        let class = parts[0];
        let lib = parts[1];
        let ver = parts[2];

        _ = writeln!(clean_classpath, "{class}:{lib}\n");

        let (url, path) = if let Some(downloads) = &library.downloads {
            (
                downloads.artifact.url.clone(),
                downloads.artifact.path.clone(),
            )
        } else {
            let base = library
                .url
                .clone()
                .unwrap_or("https://libraries.minecraft.net/".to_owned());
            let path = format!("{}/{lib}/{ver}/{lib}-{ver}.jar", class.replace('.', "/"));

            let url = base + &path;
            (url, path)
        };

        _ = write!(classpath, "../forge/libraries/{path}");
        classpath.push(CLASSPATH_SEPARATOR);

        let file_path = libraries_path.join(&path);
        if file_path.exists() {
            continue;
        }

        let dir_path = file_path.parent().unwrap();
        fs::create_dir_all(dir_path).await.path(dir_path)?;

        // WTF: I am NOT dealing with the unpack200 augmented library NONSENSE
        // because I haven't seen the launcher using it ONCE.
        // Please open an issue if you actually need it.
        file_utils::download_file_to_path(&url, false, &file_path).await?;
    }

    let classpath_path = neoforge_dir.join("classpath.txt");
    fs::write(&classpath_path, &classpath)
        .await
        .path(&classpath_path)?;

    let classpath_path = neoforge_dir.join("clean_classpath.txt");
    fs::write(&classpath_path, &clean_classpath)
        .await
        .path(&classpath_path)?;
    Ok(())
}

async fn get_installer(
    f_progress: Option<&mut Sender<ForgeProgress>>,
    neoforge_version: &str,
) -> Result<Vec<u8>, ForgeInstallError> {
    pt!("Downloading installer");
    send_progress(f_progress, ForgeProgress::P3DownloadingInstaller).await;
    let installer_url = format!("https://maven.neoforged.net/releases/net/neoforged/neoforge/{neoforge_version}/neoforge-{neoforge_version}-installer.jar");
    Ok(file_utils::download_file_to_bytes(&installer_url, false).await?)
}

async fn get_version_and_json(
    neoforge_version: Option<String>,
    instance: &InstanceSelection,
    f_progress: Option<&mut Sender<ForgeProgress>>,
) -> Result<(String, VersionDetails), ForgeInstallError> {
    Ok(if let Some(n) = neoforge_version {
        (n, VersionDetails::load(instance).await?)
    } else {
        pt!("Checking NeoForge versions");
        send_progress(f_progress, ForgeProgress::P2DownloadingJson).await;
        let (versions, version_json) = get_versions(instance.clone()).await?;

        let neoforge_version = versions
            .last()
            .ok_or(ForgeInstallError::NoForgeVersionFound)?
            .clone();

        (neoforge_version, version_json)
    })
}

async fn get_version_json(
    installer_bytes: &[u8],
    neoforge_dir: &Path,
    json: &VersionDetails,
) -> Result<ql_core::json::forge::JsonDetails, ForgeInstallError> {
    let mut zip = zip::ZipArchive::new(Cursor::new(installer_bytes))?;

    let mut file = zip
        .by_name("version.json")
        .map_err(|_| ForgeInstallError::NoInstallJson(json.get_id().to_owned()))?;
    let forge_json = std::io::read_to_string(&mut file)
        .map_err(|n| ForgeInstallError::ZipIoError(n, "version.json".to_owned()))?;

    let out_jar_version_path = neoforge_dir.join("details.json");
    fs::write(&out_jar_version_path, &forge_json)
        .await
        .path(&out_jar_version_path)?;

    let jar_version_json: ql_core::json::forge::JsonDetails =
        serde_json::from_str(&forge_json).json(forge_json)?;

    Ok(jar_version_json)
}

async fn send_progress(f_progress: Option<&mut Sender<ForgeProgress>>, message: ForgeProgress) {
    if let Some(progress) = f_progress {
        progress.send(message).await;
    }
}

pub async fn get_versions(
    instance_selection: InstanceSelection,
) -> Result<(Vec<String>, VersionDetails), ForgeInstallError> {
    let versions: NeoforgeVersions =
        file_utils::download_file_to_json(NEOFORGE_VERSIONS_URL, false).await?;

    let version_json = VersionDetails::load(&instance_selection).await?;
    let release_time = DateTime::parse_from_rfc3339(&version_json.releaseTime)?;

    let v1_20_2 = DateTime::parse_from_rfc3339("2023-09-20T09:02:57+00:00")?;
    if release_time < v1_20_2 {
        return Err(ForgeInstallError::NeoforgeOutdatedMinecraft);
    }

    let version = version_json.get_id();
    let start_pattern = if REGEX_SNAPSHOT.is_match(version) {
        // Snapshot version
        format!("0.{version}.")
    } else {
        // Release version
        let mut start_pattern = version[2..].to_owned();
        if !start_pattern.contains('.') {
            // "20" (in 1.20) -> "20.0" (in 1.20.0)
            // Ensures there are a constant number of parts in the version.
            start_pattern.push_str(".0");
        }
        start_pattern.push('.');
        start_pattern
    };

    let versions: Vec<String> = versions
        .versions
        .iter()
        .filter(|n| n.starts_with(&start_pattern))
        .cloned()
        .collect();
    if versions.is_empty() {
        return Err(ForgeInstallError::NoForgeVersionFound);
    }

    Ok((versions, version_json))
}

async fn delete(dir: &Path, path: &str) -> Result<(), IoError> {
    let delete_path = dir.join(path);
    if delete_path == dir || path.trim().is_empty() {
        return Ok(());
    }
    if delete_path.exists() {
        fs::remove_file(&delete_path).await.path(delete_path)?;
    }
    Ok(())
}

async fn create_required_jsons(neoforge_dir: &Path) -> Result<(), ForgeInstallError> {
    let p = neoforge_dir.join("launcher_profiles.json");
    fs::write(&p, "{}").await.path(p)?;
    let p = neoforge_dir.join("launcher_profiles_microsoft_store.json");
    fs::write(&p, "{}").await.path(p)?;

    Ok(())
}

const FORGE_INSTALLER_CLIENT: &[u8] =
    include_bytes!("../../../../assets/installers/NeoForgeInstallerClient.class");
const FORGE_INSTALLER_SERVER: &[u8] =
    include_bytes!("../../../../assets/installers/NeoForgeInstallerServer.class");

pub async fn run_installer(
    neoforge_dir: &Path,
    mut progress: Option<Sender<ForgeProgress>>,
    is_server: bool,
) -> Result<(), ForgeInstallError> {
    pt!("Running Installer");
    send_progress(progress.as_mut(), ForgeProgress::P5RunningInstaller).await;

    let installer = if is_server {
        FORGE_INSTALLER_SERVER
    } else {
        FORGE_INSTALLER_CLIENT
    };
    let installer_class = neoforge_dir.join("NeoForgeInstaller.class");
    fs::write(&installer_class, installer)
        .await
        .path(installer_class)?;

    let java_path = pipe_progress_ext::<GenericProgress, ForgeProgress, _, _, _>(
        progress,
        |j_progress| get_java_binary(JavaVersion::Java21, JAVA, j_progress),
        ForgeProgress::P4InstallingJava,
    )
    .await?;
    let mut command = Command::new(&java_path);
    no_window!(command);
    command
        .args([
            "-cp",
            &format!(
                "forge/{INSTALLER_NAME}{CLASSPATH_SEPARATOR}{INSTALLER_NAME}{CLASSPATH_SEPARATOR}forge/{CLASSPATH_SEPARATOR}."
            ),
            "NeoForgeInstaller",
        ])
        .current_dir(if is_server {
            neoforge_dir
                .parent()
                .map_or(neoforge_dir.join(".."), |n| n.to_owned())
        } else {
            neoforge_dir.to_owned()
        });

    let output = command.output().await.path(java_path)?;
    if !output.status.success() {
        return Err(ForgeInstallError::InstallerError(
            String::from_utf8(output.stdout)?,
            String::from_utf8(output.stderr)?,
        ));
    }
    Ok(())
}
