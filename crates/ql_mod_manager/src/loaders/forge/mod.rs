use error::Is404NotFound;
use owo_colors::OwoColorize;
use ql_core::{
    do_jobs, err, file_utils, info,
    json::{
        forge::{JsonDetails, JsonDetailsLibrary, JsonInstallProfile, JsonVersions},
        instance_config::ModTypeInfo,
        VersionDetails,
    },
    pipe_progress_ext, pt, GenericProgress, InstanceSelection, IntoIoError, IntoJsonError, IoError,
    Loader, Progress, CLASSPATH_SEPARATOR,
};
use ql_java_handler::{get_java_binary, JavaVersion, JAVA};
use sipper::Sender;
use std::{
    fmt::Write,
    io::Cursor,
    path::{Path, PathBuf},
    process::Command,
};
use tokio::{fs, sync::Mutex};

use crate::loaders::{change_instance_type, FORGE_INSTALLER_CLIENT, FORGE_INSTALLER_SERVER};

mod error;
mod server;
pub use server::install_server;
mod uninstall;

pub use error::ForgeInstallError;
pub use uninstall::uninstall;

struct ForgeInstaller {
    progress: Option<Sender<ForgeProgress>>,

    version: String,
    norm_forge_version: String,
    short_version: String,
    major_version: usize,

    instance_dir: PathBuf,
    forge_dir: PathBuf,
    is_server: bool,
    version_json: VersionDetails,
}

impl ForgeInstaller {
    pub async fn delete(&self, path: &str) -> Result<(), IoError> {
        let delete_path = self.forge_dir.join(path);
        if delete_path.exists() {
            fs::remove_file(&delete_path).await.path(delete_path)?;
        }
        Ok(())
    }

    async fn new(
        forge_version: Option<String>, // example: "11.15.1.2318" for 1.8.9
        mut f_progress: Option<Sender<ForgeProgress>>,
        instance: InstanceSelection,
    ) -> Result<Self, ForgeInstallError> {
        let instance_dir = instance.get_instance_path();
        let forge_dir = if instance.is_server() {
            instance_dir.clone()
        } else {
            get_forge_dir(&instance_dir).await?
        };

        let version_json = VersionDetails::load(&instance).await?;
        let minecraft_version = version_json.get_id();

        create_mods_dir(&instance_dir).await?;

        pt!("Downloading JSON");
        if let Some(progress) = &mut f_progress {
            progress.send(ForgeProgress::P2DownloadingJson).await;
        }

        let version = if let Some(n) = forge_version {
            n
        } else {
            get_forge_version(minecraft_version).await?
        };

        pt!("{}: {version}", "Version".underline());

        let norm_version = {
            let number_of_full_stops = minecraft_version.chars().filter(|c| *c == '.').count();
            if number_of_full_stops == 1 {
                format!("{minecraft_version}.0")
            } else {
                minecraft_version.to_owned()
            }
        };
        let short_version = format!("{minecraft_version}-{version}");
        let norm_forge_version = format!("{short_version}-{norm_version}");
        let major_version: usize = version.split('.').next().unwrap_or(&version).parse()?;

        Ok(Self {
            progress: f_progress,

            version,
            norm_forge_version,
            short_version,
            major_version,

            instance_dir,
            forge_dir,
            is_server: instance.is_server(),
            version_json,
        })
    }

    async fn download_forge_installer(
        &mut self,
    ) -> Result<(Vec<u8>, String, PathBuf), ForgeInstallError> {
        let (file_type, file_type_flipped) = if self.major_version < 14 {
            ("universal", "installer")
        } else {
            ("installer", "universal")
        };

        info!("Downloading Installer");
        self.send_progress(ForgeProgress::P3DownloadingInstaller)
            .await;

        let installer_file = self.try_downloading_from_urls(&[
            &format!("https://files.minecraftforge.net/maven/net/minecraftforge/forge/{ver}/forge-{ver}-{file_type}.jar", ver = self.short_version),
            &format!("https://files.minecraftforge.net/maven/net/minecraftforge/forge/{ver}/forge-{ver}-{file_type}.jar", ver = self.norm_forge_version),
            &format!("https://files.minecraftforge.net/maven/net/minecraftforge/forge/{ver}/forge-{ver}-{file_type_flipped}.jar", ver = self.short_version),
            &format!("https://files.minecraftforge.net/maven/net/minecraftforge/forge/{ver}/forge-{ver}-{file_type_flipped}.jar", ver = self.norm_forge_version),
            // Minecraft 1.1 to 1.5.1: Install as jarmod
            &format!("https://files.minecraftforge.net/maven/net/minecraftforge/forge/{}/forge-{}-client.zip", self.short_version, self.short_version),
            &format!("https://files.minecraftforge.net/maven/net/minecraftforge/forge/{}/forge-{}-client.zip", self.norm_forge_version, self.norm_forge_version),
            // TODO: Use <https://maven.minecraftforge.net/net/minecraftforge/forge/1.5.2-7.8.1.738/forge-1.5.2-7.8.1.738-installer.jar>
            &format!("https://maven.minecraftforge.net/net/minecraftforge/forge/{}/forge-{}-universal.zip", self.short_version, self.short_version),
            &format!("https://maven.minecraftforge.net/net/minecraftforge/forge/{}/forge-{}-universal.zip", self.norm_forge_version, self.norm_forge_version),
        ]).await?;

        let installer_name = format!("forge-{}-{file_type}.jar", self.short_version);
        let installer_path = self.forge_dir.join(&installer_name);
        fs::write(&installer_path, &installer_file)
            .await
            .path(&installer_path)?;
        Ok((installer_file, installer_name, installer_path))
    }

    async fn send_progress(&mut self, message: ForgeProgress) {
        if let Some(progress) = &mut self.progress {
            progress.send(message).await;
        }
    }

    async fn try_downloading_from_urls(&self, urls: &[&str]) -> Result<Vec<u8>, ForgeInstallError> {
        let num_urls = urls.len();
        for (i, url) in urls.iter().enumerate() {
            let result = file_utils::download_file_to_bytes(url, false).await;

            return match result {
                Ok(file) => {
                    pt!("{}: {}", "Url".underline(), url.bright_black());
                    Ok(file)
                }
                Err(err) => {
                    let is_last_url = i + 1 == num_urls;
                    if err.is_not_found() && !is_last_url {
                        continue;
                    }
                    Err(ForgeInstallError::Request(err))
                }
            };
        }
        panic!("Forge installer: Reached invalid state (while retrying downloads)")
    }

    async fn run_installer_and_get_classpath(
        &mut self,
        installer_name: &str,
    ) -> Result<(PathBuf, String), ForgeInstallError> {
        let libraries_dir = self.forge_dir.join("libraries");
        fs::create_dir_all(&libraries_dir)
            .await
            .path(&libraries_dir)?;

        let classpath = if self.major_version >= 14 {
            // 1.12+
            self.run_installer(installer_name).await?;

            if self.major_version < 39 {
                // 1.12 - 1.18
                format!(
                    "../forge/libraries/net/minecraftforge/forge/{}/forge-{}.jar{CLASSPATH_SEPARATOR}",
                    self.short_version,
                    self.short_version
                )
            } else {
                // 1.18.1+
                String::new()
            }
        } else {
            // 1.1 - 1.11.2
            format!("../forge/{installer_name}{CLASSPATH_SEPARATOR}")
        };
        Ok((libraries_dir, classpath))
    }

    async fn run_installer(&mut self, installer_name: &str) -> Result<(), ForgeInstallError> {
        let installer = if self.is_server {
            FORGE_INSTALLER_SERVER
        } else {
            FORGE_INSTALLER_CLIENT
        };
        let installer_class = self.forge_dir.join("ForgeInstaller.class");
        fs::write(&installer_class, installer)
            .await
            .path(installer_class)?;

        self.run_installer_create_garbage_files().await?;

        let java_version = if cfg!(target_os = "windows") {
            // WTF: No clue why this is needed but it won't work without this.
            // Hey, that's what you get for not using PrismLauncher!
            self.version_json
                .javaVersion
                .clone()
                .map(JavaVersion::from)
                .unwrap_or(JavaVersion::Java21)
        } else {
            JavaVersion::Java8
        };
        let java_path = pipe_progress_ext::<GenericProgress, ForgeProgress, _, _, _>(
            self.progress.clone(),
            |j_progress| get_java_binary(java_version, JAVA, j_progress),
            ForgeProgress::P4InstallingJava,
        )
        .await?;
        info!("Running Installer...");
        self.send_progress(ForgeProgress::P5RunningInstaller).await;
        let mut command = Command::new(&java_path);
        pt!(
            "{}: {:?}",
            "Install Path".underline(),
            self.forge_dir.bright_black()
        );
        command
            .args([
                "-cp",
                &format!("{installer_name}{CLASSPATH_SEPARATOR}."),
                "ForgeInstaller",
            ])
            .current_dir(&self.forge_dir);

        let output = command.output().path(java_path)?;
        if !output.status.success() {
            return Err(ForgeInstallError::InstallerError(
                String::from_utf8(output.stdout)?,
                String::from_utf8(output.stderr)?,
            ));
        }
        Ok(())
    }

    async fn run_installer_create_garbage_files(&self) -> Result<(), ForgeInstallError> {
        if !self.is_server {
            let launcher_profiles_json_path = self.forge_dir.join("launcher_profiles.json");
            fs::write(&launcher_profiles_json_path, "{}")
                .await
                .path(launcher_profiles_json_path)?;
            let launcher_profiles_json_microsoft_store_path = self
                .forge_dir
                .join("launcher_profiles_microsoft_store.json");
            fs::write(&launcher_profiles_json_microsoft_store_path, "{}")
                .await
                .path(launcher_profiles_json_microsoft_store_path)?;
        }
        Ok(())
    }

    fn get_forge_json(
        &self,
        installer_file: &[u8],
    ) -> Result<(JsonDetails, String), ForgeInstallError> {
        let mut zip = zip::ZipArchive::new(Cursor::new(installer_file))?;

        if let Ok(mut file) = zip.by_name("version.json") {
            let forge_json = std::io::read_to_string(&mut file);
            let forge_json = forge_json
                .map_err(|n| ForgeInstallError::ZipIoError(n, "version.json".to_owned()))?;

            let forge_json_parsed: JsonDetails =
                serde_json::from_str(&forge_json).json(forge_json.clone())?;

            return Ok((forge_json_parsed, forge_json));
        }
        if let Ok(mut file) = zip.by_name("install_profile.json") {
            let forge_json = std::io::read_to_string(&mut file);
            let forge_json = forge_json
                .map_err(|n| ForgeInstallError::ZipIoError(n, "install_profile.json".to_owned()))?;

            return if let Ok(forge_json_parsed) = serde_json::from_str(&forge_json) {
                let forge_json_parsed: JsonInstallProfile = forge_json_parsed;

                let to_string = serde_json::to_string(&forge_json_parsed.versionInfo)
                    .unwrap_or(forge_json.clone());
                Ok((forge_json_parsed.versionInfo, to_string))
            } else {
                let forge_json_parsed: JsonDetails =
                    serde_json::from_str(&forge_json).json(forge_json.clone())?;
                Ok((forge_json_parsed, forge_json))
            };
        }
        Err(ForgeInstallError::NoInstallJson(
            self.version_json.get_id().to_owned(),
        ))
    }

    async fn download_library(
        &self,
        library: JsonDetailsLibrary,
        library_i: &Mutex<usize>,
        num_libraries: usize,
        libraries_dir: &Path,
        classpath: &Mutex<String>,
        clean_classpath: &Mutex<String>,
    ) -> Result<(), ForgeInstallError> {
        let parts: Vec<&str> = library.name.split(':').collect();
        let class = parts[0];
        let lib = parts[1];
        let ver = parts[2];

        _ = writeln!(clean_classpath.lock().await, "{class}:{lib}");

        let (file, path) = Self::get_filename_and_path(lib, ver, &library, class)?;

        if class == "net.minecraftforge" && lib == "forge" && self.major_version < 49 {
            pt!("(_/{num_libraries}): built-in forge library, skipping...");
            return Ok(());
        }

        let url = if let Some(downloads) = &library.downloads {
            downloads.artifact.url.clone()
        } else {
            let baseurl = if let Some(url) = &library.url {
                url.to_owned()
            } else {
                "https://libraries.minecraft.net/".to_owned()
            };
            format!("{baseurl}{path}/{file}")
        };

        let lib_dir_path = libraries_dir.join(&path);
        fs::create_dir_all(&lib_dir_path)
            .await
            .path(&lib_dir_path)?;

        let dest = lib_dir_path.join(&file);
        if !dest.exists() {
            let result = file_utils::download_file_to_path(&url, false, &dest).await;
            if result.is_not_found() {
                err!("Error 404 not found. Skipping...");
                return Ok(());
            }
            result?;
        }

        {
            let mut i = library_i.lock().await;
            *i += 1;
            pt!("({}/{num_libraries}): {}", *i, library.name.bright_black());

            if let Some(mut progress) = self.progress.clone() {
                progress
                    .send(ForgeProgress::P6DownloadingLibrary {
                        num: *i,
                        out_of: num_libraries,
                    })
                    .await;
            }
        }

        Self::add_to_classpath(classpath, &path, &file).await;

        Ok(())
    }

    async fn add_to_classpath(classpath: &Mutex<String>, path: &str, file: &str) {
        let classpath_item = format!("../forge/libraries/{path}/{file}{CLASSPATH_SEPARATOR}");
        // println!("adding library to classpath {classpath_item}");
        classpath.lock().await.push_str(&classpath_item);
    }

    fn get_filename_and_path(
        lib: &str,
        ver: &str,
        library: &JsonDetailsLibrary,
        class: &str,
    ) -> Result<(String, String), ForgeInstallError> {
        let (file, path) = if let Some(downloads) = &library.downloads {
            let parent = PathBuf::from(&downloads.artifact.path)
                .parent()
                .ok_or(ForgeInstallError::LibraryParentError)?
                .to_owned();
            (
                PathBuf::from(&downloads.artifact.path)
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .to_string(),
                parent
                    .to_str()
                    .ok_or(ForgeInstallError::PathBufToStr(parent.clone()))?
                    .to_owned(),
            )
        } else {
            (
                format!("{lib}-{ver}.jar"),
                format!("{}/{lib}/{ver}", class.replace('.', "/")),
            )
        };
        Ok((file, path))
    }
}

async fn get_forge_version(minecraft_version: &str) -> Result<String, ForgeInstallError> {
    let json = JsonVersions::download().await?;
    let version = json
        .get_forge_version(minecraft_version)
        .ok_or(ForgeInstallError::NoForgeVersionFound)?;
    Ok(version)
}

async fn get_forge_dir(instance_dir: &Path) -> Result<PathBuf, ForgeInstallError> {
    let forge_dir = instance_dir.join("forge");
    fs::create_dir_all(&forge_dir).await.path(&forge_dir)?;
    Ok(forge_dir)
}

async fn create_mods_dir(instance_dir: &Path) -> Result<(), ForgeInstallError> {
    let mods_dir_path = instance_dir.join(".minecraft/mods");
    fs::create_dir_all(&mods_dir_path)
        .await
        .path(mods_dir_path)?;
    Ok(())
}

pub async fn install(
    forge_version: Option<String>, // example: "11.15.1.2318" for 1.8.9
    instance: InstanceSelection,
    progress: Option<Sender<ForgeProgress>>,
) -> Result<(), ForgeInstallError> {
    match instance {
        InstanceSelection::Instance(name) => install_client(forge_version, name, progress).await,
        InstanceSelection::Server(name) => install_server(forge_version, name, progress).await,
    }
}

#[derive(Default, Clone, Debug)]
pub enum ForgeProgress {
    #[default]
    P1Start,
    P2DownloadingJson,
    P3DownloadingInstaller,
    P4InstallingJava(GenericProgress),
    P5RunningInstaller,
    P6DownloadingLibrary {
        num: usize,
        out_of: usize,
    },
    P7Done,
}

impl Progress for ForgeProgress {
    fn get_num(&self) -> f32 {
        match self {
            Self::P1Start | Self::P2DownloadingJson => 0.0,
            Self::P3DownloadingInstaller => 1.0,
            Self::P5RunningInstaller => 2.0,
            Self::P6DownloadingLibrary { num, out_of } => {
                4.0 + (*num as f32 * 2.0 / *out_of as f32)
            }
            Self::P7Done => 8.0,
            Self::P4InstallingJava(p) => p.get_num(),
        }
    }

    fn get_message(&self) -> Option<String> {
        Some(match self {
            Self::P1Start => "Installing forge...".to_owned(),
            Self::P2DownloadingJson => "Downloading JSON".to_owned(),
            Self::P3DownloadingInstaller => "Downloading installer".to_owned(),
            Self::P5RunningInstaller => "Running Installer (this might take a while)".to_owned(),
            Self::P6DownloadingLibrary { num, out_of } => {
                format!("Downloading Library ({num}/{out_of})")
            }
            Self::P7Done => "Done!".to_owned(),
            Self::P4InstallingJava(p) => p.get_message()?,
        })
    }

    fn total(&self) -> f32 {
        8.0
    }
}

pub async fn install_client(
    forge_version: Option<String>,
    instance_name: String,
    mut progress: Option<Sender<ForgeProgress>>,
) -> Result<(), ForgeInstallError> {
    info!("Started installing forge");
    if let Some(progress) = &mut progress {
        progress.send(ForgeProgress::P1Start).await;
    }

    let mut installer = ForgeInstaller::new(
        forge_version,
        progress,
        InstanceSelection::Instance(instance_name.clone()),
    )
    .await?;

    let (installer_file, installer_name, _) = installer.download_forge_installer().await?;
    if installer.version_json.is_legacy_version() && installer.version_json.get_id() != "1.5.2" {
        ql_core::jarmod::insert(
            InstanceSelection::Instance(instance_name.clone()),
            installer_file,
            "Forge",
        )
        .await?;
        return Ok(());
    }

    let (libraries_dir, classpath) = installer
        .run_installer_and_get_classpath(&installer_name)
        .await?;

    let classpath = Mutex::new(classpath);
    let clean_classpath = Mutex::new(String::new());

    let (forge_json, forge_json_str) = installer.get_forge_json(&installer_file)?;

    info!("Downloading libraries...");
    let libs: Vec<JsonDetailsLibrary> = forge_json
        .libraries
        .into_iter()
        .filter(|library| !matches!(library.clientreq, Some(false)))
        .collect();
    let num_libraries = libs.len();
    let library_i = Mutex::new(0);
    let jobs: Vec<_> = libs
        .into_iter()
        .map(|library| {
            installer.download_library(
                library.clone(),
                &library_i,
                num_libraries,
                &libraries_dir,
                &classpath,
                &clean_classpath,
            )
        })
        .collect();
    do_jobs(jobs.into_iter()).await?;

    let classpath_path = installer.forge_dir.join("classpath.txt");
    let classpath = classpath.lock().await.clone();
    fs::write(&classpath_path, classpath)
        .await
        .path(classpath_path)?;

    let clean_classpath_path = installer.forge_dir.join("clean_classpath.txt");
    let clean_classpath = clean_classpath.lock().await.clone();
    fs::write(&clean_classpath_path, clean_classpath)
        .await
        .path(clean_classpath_path)?;

    let json_path = installer.forge_dir.join("details.json");
    fs::write(
        &json_path,
        serde_json::to_string(&forge_json_str).json_to()?,
    )
    .await
    .path(json_path)?;

    change_instance_type(
        &installer.instance_dir,
        Loader::Forge,
        Some(ModTypeInfo {
            version: Some(installer.version.clone()),
            backend_implementation: None,
            optifine_jar: None,
        }),
    )
    .await?;

    info!("Finished installing forge");
    Ok(())
}
