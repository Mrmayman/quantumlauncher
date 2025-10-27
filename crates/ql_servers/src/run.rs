use std::{
    path::{Path, PathBuf},
    process::Stdio,
    sync::{mpsc::Sender, Arc, Mutex},
};

use ql_core::{
    err, find_forge_shim_file, info,
    json::{InstanceConfigJson, VersionDetails},
    no_window, pt, GenericProgress, InstanceSelection, IntoIoError, LaunchedProcess, LAUNCHER_DIR,
};
use ql_java_handler::{get_java_binary, JavaVersion};
use tokio::process::Command;

use crate::ServerError;

/// Runs a server.
///
/// # Arguments
/// - `name` - The name of the server to run.
/// - `java_install_progress` - The channel to send progress updates to
///   if Java needs to be installed.
///
/// # Returns
/// - `Ok((Child, bool))` - The child process and whether the server is a classic server.
/// - `Err(ServerError)` - The error that occurred.
///
/// # Errors
/// - Instance `config.json` couldn't be read or parsed
/// - Instance `details.json` couldn't be read or parsed
/// - Java binary path could not be obtained
/// - Java could not be installed (if not found)
/// - `Command` couldn't be spawned (IO Error)
/// - Forge shim file (`forge-*-shim.jar`) couldn't be found
/// - Other stuff I'm too dumb to see
pub async fn run(
    name: String,
    java_install_progress: Option<Sender<GenericProgress>>,
) -> Result<LaunchedProcess, ServerError> {
    let launcher = ServerLauncher::new(&name).await?;

    let server_jar_path = launcher.get_server_jar().await?;

    let java_path = launcher.get_java(java_install_progress.as_ref()).await?;

    let java_args = launcher.get_java_args(&server_jar_path).await?;
    let mut game_args = launcher.config.game_args.clone().unwrap_or_default();
    game_args.push("nogui".to_owned());

    info!("Java: {java_path:?}\n");
    info!("Java args: {java_args:?}\n");
    info!("Server args: {game_args:?}\n");

    let mut command = Command::new(java_path);
    command
        .args(java_args.iter().chain(game_args.iter()))
        .current_dir(&launcher.dir)
        .kill_on_drop(true);

    if launcher.config.enable_logger.unwrap_or(true) {
        no_window!(command);
        command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::piped());
    }

    let child = command.spawn().path(server_jar_path)?;
    if let Some(id) = child.id() {
        pt!("PID: {id}");
    } else {
        pt!("No ID found!");
    }
    Ok(LaunchedProcess {
        child: Arc::new(Mutex::new(child)),
        instance: InstanceSelection::Server(name),
        is_classic_server: launcher.is_classic_server(),
    })
}

struct ServerLauncher {
    dir: PathBuf,
    version_json: VersionDetails,
    config: InstanceConfigJson,
}

impl ServerLauncher {
    pub async fn new(name: &str) -> Result<Self, ServerError> {
        let dir = LAUNCHER_DIR.join("servers").join(name);
        Ok(Self {
            version_json: VersionDetails::load_from_path(&dir).await?,
            config: InstanceConfigJson::read_from_dir(&dir).await?,
            dir,
        })
    }

    pub fn is_neoforge(&self) -> bool {
        self.config.mod_type == "NeoForge"
    }

    pub fn is_classic_server(&self) -> bool {
        self.config.is_classic_server.unwrap_or_default()
    }

    pub async fn get_java(
        &self,
        java_install_progress: Option<&Sender<GenericProgress>>,
    ) -> Result<PathBuf, ServerError> {
        let version = if let Some(version) = self.version_json.javaVersion.clone() {
            version.into()
        } else {
            JavaVersion::Java8
        };

        if let Some(java_path) = &self.config.java_override {
            if !java_path.is_empty() {
                let java_path = PathBuf::from(java_path);
                if java_path.exists() {
                    return Ok(java_path);
                }
                err!("Java override at {java_path:?} does not exist!");
            }
        }
        let path = get_java_binary(version, "java", java_install_progress).await?;
        Ok(path)
    }

    pub async fn get_server_jar(&self) -> Result<PathBuf, ServerError> {
        Ok(if let Some(custom_jar) = &self.config.custom_jar {
            // Should I prioritise Fabric/Forge/Paper over a custom JAR?
            PathBuf::from(&custom_jar.name)
        } else if self.config.mod_type == "Fabric" || self.config.mod_type == "Quilt" {
            self.dir.join("fabric-server-launch.jar")
        } else if self.config.mod_type == "Forge" {
            find_forge_shim_file(&self.dir)
                .await
                .ok_or(ServerError::NoForgeShimFound)?
        } else if self.config.mod_type == "Paper" {
            self.dir.join("paper_server.jar")
        } else {
            self.dir.join("server.jar")
        })
    }

    pub async fn get_java_args(&self, jar: &Path) -> Result<Vec<String>, ServerError> {
        let mut java_args: Vec<String> = self.config.get_java_args(&[]);
        java_args.push(self.config.get_ram_argument());
        if self.config.mod_type == "Forge" {
            java_args.push("-Djava.net.preferIPv6Addresses=system".to_owned());
        } else if self.config.mod_type == "Fabric" {
            if let Some(info) = self
                .config
                .mod_type_info
                .as_ref()
                .and_then(|n| n.backend_implementation.as_ref())
            {
                // Fixes the crash:
                // Exception in thread "main" java.lang.RuntimeException: Failed to setup Fabric server environment!
                // ...
                // Caused by: java.lang.RuntimeException: net.fabricmc.loader.api.VersionParsingException: Could not parse version number component 'server'!

                if info == "Fabric (Cursed Legacy)" {
                    java_args.push(format!(
                        "-Dfabric.gameVersion={}",
                        self.version_json.get_id()
                    ));
                }
            }
        } else if self.is_neoforge() {
            #[cfg(target_family = "unix")]
            const FILENAME: &str = "unix_args.txt";
            #[cfg(target_os = "windows")]
            const FILENAME: &str = "win_args.txt";
            #[cfg(not(any(target_family = "unix", target_os = "windows")))]
            const FILENAME: &str = "YOUR_OS_IS_UNSUPPORTED";

            let mut args_path = self.dir.join("libraries/net/neoforged/neoforge");
            if let Some(ver) = self
                .config
                .mod_type_info
                .as_ref()
                .and_then(|n| n.version.as_deref())
            {
                args_path = args_path.join(ver);
            }
            let args_path = args_path.join(FILENAME);

            let args = tokio::fs::read_to_string(&args_path)
                .await
                .path(args_path)?;
            java_args.extend(
                args.lines()
                    .flat_map(str::split_whitespace)
                    .filter(|l| !l.is_empty())
                    .map(|n| n.to_owned()),
            );
        }

        let is_cl_sr = self.is_classic_server();
        if !self.is_neoforge() {
            java_args.push(if is_cl_sr { "-cp" } else { "-jar" }.to_owned());
            java_args.push(
                jar.to_str()
                    .ok_or(ServerError::PathBufToStr(jar.to_owned()))?
                    .to_owned(),
            );
        }

        if is_cl_sr {
            java_args.push("com.mojang.minecraft.server.MinecraftServer".to_owned());
        }

        Ok(java_args)
    }
}
