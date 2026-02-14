use crate::{
    auth::{ms::CLIENT_ID, AccountData, AccountType},
    download::GameDownloader,
    jarmod,
};
use ql_core::{
    err, file_utils, info,
    json::{
        forge, lwjgl, version::Library, FabricJSON, GlobalSettings, InstanceConfigJson,
        JsonOptifine, VersionDetails, V_1_12_2, V_1_5_2, V_PAULSCODE_LAST, V_PRECLASSIC_LAST,
    },
    pt, GenericProgress, InstanceSelection, IntoIoError, IntoJsonError, IoError, JsonFileError,
    Loader, CLASSPATH_SEPARATOR, LAUNCHER_DIR,
};
use ql_java_handler::{get_java_binary, JavaVersion};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    process::Stdio,
    sync::mpsc::Sender,
};
use tokio::process::Command;

use super::{error::GameLaunchError, replace_var};

pub struct GameLauncher {
    username: String,
    instance_name: String,

    /// If Java isn't installed, it will be auto-installed by the launcher.
    /// This field allows you to send progress updates
    /// to the GUI during installation.
    java_install_progress_sender: Option<Sender<GenericProgress>>,

    /// Progress updates during launch preparation (e.g. custom LWJGL downloads).
    launch_progress_sender: Option<Sender<GenericProgress>>,

    /// Client: `QuantumLauncher/instances/NAME/`
    /// Server: `QuantumLauncher/servers/NAME/`
    pub instance_dir: PathBuf,
    /// Client: `QuantumLauncher/instances/NAME/.minecraft/`
    /// Server: `QuantumLauncher/servers/NAME/`
    pub minecraft_dir: PathBuf,

    pub config: InstanceConfigJson,
    pub version_json: VersionDetails,
    /// Launcher-wide instance settings. These
    /// can be overridden by `config_json.global_settings`.
    global_settings: Option<GlobalSettings>,
    extra_java_args: Vec<String>,
}

impl GameLauncher {
    pub async fn new(
        instance_name: String,
        username: String,
        java_install_progress_sender: Option<Sender<GenericProgress>>,
        launch_progress_sender: Option<Sender<GenericProgress>>,
        global_settings: Option<GlobalSettings>,
        extra_java_args: Vec<String>,
    ) -> Result<Self, GameLaunchError> {
        let instance_dir = get_instance_dir(&instance_name).await?;

        let minecraft_dir = instance_dir.join(".minecraft");
        tokio::fs::create_dir_all(&minecraft_dir)
            .await
            .path(&minecraft_dir)?;

        let config = InstanceConfigJson::read_from_dir(&instance_dir).await?;

        let instance = InstanceSelection::Instance(instance_name.clone());
        let mut version_json = VersionDetails::load(&instance).await?;
        version_json.apply_tweaks(&instance).await?;

        Ok(Self {
            username,
            instance_name,
            java_install_progress_sender,
            launch_progress_sender,
            instance_dir,
            minecraft_dir,
            config,
            version_json,
            global_settings,
            extra_java_args,
        })
    }

    pub fn init_game_arguments(
        &mut self,
        account_details: Option<&AccountData>,
    ) -> Result<Vec<String>, GameLaunchError> {
        let mut game_arguments: Vec<String> =
            if let Some(arguments) = &self.version_json.minecraftArguments {
                arguments.split(' ').map(ToOwned::to_owned).collect()
            } else if let Some(arguments) = &self.version_json.arguments {
                arguments
                    .game
                    .iter()
                    .filter_map(|arg| arg.as_str())
                    .map(ToOwned::to_owned)
                    .collect()
            } else {
                return Err(GameLaunchError::VersionJsonNoArgumentsField(Box::new(
                    self.version_json.clone(),
                )));
            };

        if let Some(account_type) = account_details.map(|n| n.account_type) {
            if matches!(account_type, AccountType::ElyBy | AccountType::LittleSkin)
                && !self.version_json.is_legacy_version()
                && !game_arguments.iter().any(|n| n.contains("uuid"))
            {
                game_arguments.push("--uuid".to_owned());
                game_arguments.push("${uuid}".to_owned());
            }
        }

        // Add custom resolution arguments if specified
        // Priority: Instance-specific setting > Global default > Minecraft default
        let (width_to_use, height_to_use) =
            self.config.get_window_size(self.global_settings.as_ref());

        if let Some(width) = width_to_use {
            game_arguments.push("--width".to_owned());
            game_arguments.push(width.to_string());
        }
        if let Some(height) = height_to_use {
            game_arguments.push("--height".to_owned());
            game_arguments.push(height.to_string());
        }

        game_arguments.extend(self.config.game_args.clone().unwrap_or_default());

        Ok(game_arguments)
    }

    pub async fn fill_game_arguments(
        &self,
        game_arguments: &mut [String],
        account_details: Option<&AccountData>,
    ) -> Result<(), GameLaunchError> {
        for arg in game_arguments.iter_mut() {
            replace_var(arg, "auth_player_name", &self.username);
            replace_var(arg, "version_name", self.version_json.get_id());
            let Some(minecraft_dir_path) = self.minecraft_dir.to_str() else {
                return Err(GameLaunchError::PathBufToString(self.minecraft_dir.clone()));
            };
            replace_var(arg, "game_directory", minecraft_dir_path);

            self.set_assets_argument(arg).await?;
            replace_var(arg, "auth_xuid", "0");

            let uuid = if let Some(account_details) = account_details {
                &account_details.uuid
            } else {
                "00000000-0000-0000-0000-000000000000"
            };
            replace_var(arg, "auth_uuid", uuid);
            replace_var(arg, "uuid", uuid);

            let access_token = if let Some(account_details) = account_details {
                account_details
                    .access_token
                    .as_ref()
                    .ok_or(GameLaunchError::InvalidToken)?
            } else {
                "0"
            };
            replace_var(arg, "auth_access_token", access_token);
            replace_var(arg, "auth_session", access_token);
            replace_var(arg, "accessToken", access_token);

            replace_var(arg, "clientid", CLIENT_ID);
            replace_var(
                arg,
                "user_type",
                if account_details.is_some() {
                    "msa"
                } else {
                    "legacy"
                },
            );
            replace_var(arg, "version_type", "release");
            replace_var(arg, "assets_index_name", &self.version_json.assetIndex.id);
            replace_var(arg, "user_properties", "{}");
        }
        Ok(())
    }

    async fn set_assets_argument(&self, argument: &mut String) -> Result<(), GameLaunchError> {
        let launcher_dir = &*LAUNCHER_DIR;

        let old_assets_path_v2 = launcher_dir
            .join("assets")
            .join(&self.version_json.assetIndex.id);

        let old_assets_path_v1 = self.instance_dir.join("assets");
        let assets_path = launcher_dir.join("assets/dir");

        if old_assets_path_v2.exists() {
            info!("Migrating old assets to new path...");
            file_utils::copy_dir_recursive(&old_assets_path_v2, &assets_path).await?;
            tokio::fs::remove_dir_all(&old_assets_path_v2)
                .await
                .path(old_assets_path_v2)?;
        }

        if old_assets_path_v1.exists() {
            migrate_to_new_assets_path(&old_assets_path_v1, &assets_path).await?;
        }

        let assets_path_fixed = if assets_path.exists() {
            assets_path
        } else {
            launcher_dir.join("assets/null")
        };
        let Some(assets_path) = assets_path_fixed.to_str() else {
            return Err(GameLaunchError::PathBufToString(assets_path_fixed));
        };
        replace_var(argument, "assets_root", assets_path);
        replace_var(argument, "game_assets", assets_path);
        Ok(())
    }

    pub async fn create_mods_dir(&self) -> Result<(), IoError> {
        let mods_dir = self.minecraft_dir.join("mods");
        tokio::fs::create_dir_all(&mods_dir).await.path(mods_dir)?;
        Ok(())
    }

    pub async fn init_java_arguments(
        &mut self,
        auth: Option<&AccountData>,
    ) -> Result<Vec<String>, GameLaunchError> {
        let natives_path = self.instance_dir.join("libraries").join("natives");
        let natives_path = natives_path
            .to_str()
            .ok_or(GameLaunchError::PathBufToString(natives_path.clone()))?;

        // TODO: deal with self.version_json.arguments.jvm (currently ignored)
        let mut args: Vec<String> = self
            .config
            .get_java_args(&self.extra_java_args)
            .into_iter()
            .filter(|arg| !arg.trim().is_empty())
            .chain([
                "-Dminecraft.launcher.brand=minecraft-launcher".to_owned(),
                "-Dminecraft.launcher.version=2.1.1349".to_owned(),
                format!("-Djava.library.path={natives_path}"),
                format!("-Djna.tmpdir={natives_path}"),
                format!("-Dorg.lwjgl.system.SharedLibraryExtractPath={natives_path}"),
                format!("-Dio.netty.native.workdir={natives_path}"),
                self.config.get_ram_argument(),
            ])
            .collect();

        if auth.is_none_or(|n| !n.is_microsoft()) && self.version_json.id.starts_with("1.16") {
            // Fixes "Multiplayer is disabled" issue on 1.16.x
            args.push("-Dminecraft.api.auth.host=https://nope.invalid".to_owned());
            args.push("-Dminecraft.api.account.host=https://nope.invalid".to_owned());
            args.push("-Dminecraft.api.session.host=https://nope.invalid".to_owned());
            args.push("-Dminecraft.api.services.host=https://nope.invalid".to_owned());
        } else if let Some(authlib) = auth.and_then(AccountData::get_authlib_url) {
            args.push(crate::auth::get_authlib_injector(authlib).await?);
        }

        if cfg!(target_pointer_width = "32") {
            args.push("-Xss1M".to_owned());
        }

        if cfg!(target_os = "macos") {
            args.push("-XstartOnFirstThread".to_owned());
        }

        self.java_arguments_betacraft(&mut args);

        Ok(args)
    }

    /// Adds BetaCraft proxy to fix missing/incorrect sounds
    /// in old versions of Minecraft.
    ///
    /// This auto adjusts the port based on version.
    #[allow(clippy::doc_markdown)]
    fn java_arguments_betacraft(&mut self, args: &mut Vec<String>) {
        if !self.version_json.is_legacy_version() {
            return;
        }

        // Backwards compatibility with Quantum Launcher v0.3.1 - v0.4.1
        #[allow(deprecated)]
        if self.config.omniarchive.is_some() {
            args.push("-Dhttp.proxyHost=betacraft.uk".to_owned());
            if self.version_json.id.starts_with("c0.") {
                // Classic
                args.push("-Dhttp.proxyPort=11701".to_owned());
            } else if self.version_json.id.starts_with("b1.9") {
                // Beta 1.9
                args.push("-Dhttp.proxyPort=11706".to_owned());
            } else if self.version_json.id.starts_with("b1.") {
                // Beta 1.0 - 1.8.1
                args.push("-Dhttp.proxyPort=11705".to_owned());
            } else if self.version_json.id.starts_with("1.") {
                // Release 1.0 - 1.5.2
                args.push("-Dhttp.proxyPort=11707".to_owned());
            } else {
                // Indev, Infdev and Alpha (mostly same)
                args.push("-Dhttp.proxyPort=11702".to_owned());
            }
        }

        // Fixes crash on old versions
        args.push("-Djava.util.Arrays.useLegacyMergeSort=true".to_owned());
    }

    pub async fn setup_fabric(
        &self,
        java_arguments: &mut Vec<String>,
        game_arguments: &mut Vec<String>,
    ) -> Result<Option<FabricJSON>, GameLaunchError> {
        if !matches!(self.config.mod_type, Loader::Fabric | Loader::Quilt) {
            return Ok(None);
        }

        let fabric_json = self.get_fabric_json().await?;
        if let Some(jvm) = fabric_json.arguments.as_ref().and_then(|n| n.jvm.as_ref()) {
            java_arguments.extend(jvm.clone());
        }

        if let Some(jvm) = fabric_json.arguments.as_ref().and_then(|n| n.game.as_ref()) {
            game_arguments.extend(jvm.clone());
        }

        Ok(Some(fabric_json))
    }

    pub async fn setup_forge(
        &mut self,
        java_arguments: &mut Vec<String>,
        game_arguments: &mut Vec<String>,
    ) -> Result<Option<forge::JsonDetails>, GameLaunchError> {
        if !matches!(self.config.mod_type, Loader::Forge | Loader::Neoforge) {
            return Ok(None);
        }
        if self.version_json.is_legacy_version() && self.version_json.get_id() != "1.5.2" {
            return Ok(None);
        }

        let json = self.get_forge_json().await?;

        if let Some(arguments) = &json.arguments {
            if let Some(jvm) = &arguments.jvm {
                java_arguments.extend(jvm.clone());
            }
            game_arguments.extend(arguments.game.clone());
        } else if let Some(arguments) = &json.minecraftArguments {
            let new: Vec<String> = arguments.split(' ').map(str::to_owned).collect();
            *game_arguments = deduplicate_game_args(game_arguments, &new);
        }
        Ok(Some(json))
    }

    async fn get_fabric_json(&self) -> Result<FabricJSON, JsonFileError> {
        let json_path = self.instance_dir.join("fabric.json");
        let fabric_json = tokio::fs::read_to_string(&json_path)
            .await
            .path(json_path)?;
        Ok(serde_json::from_str(&fabric_json).json(fabric_json)?)
    }

    async fn get_forge_json(&self) -> Result<forge::JsonDetails, JsonFileError> {
        let json_path = self.instance_dir.join("forge/details.json");
        let json = tokio::fs::read_to_string(&json_path)
            .await
            .path(json_path)?;
        let json_details: forge::JsonDetails = match serde_json::from_str(&json) {
            Ok(n) => n,
            Err(err) => {
                if err.to_string().starts_with("invalid type: string") {
                    // Sometimes the "JSON" is formatted like
                    // "{\"hello\" : \"world\"}"
                    // See those pesky backslashed quotes?
                    // We fix that here.
                    let json_details: String = serde_json::from_str(&json).json(json)?;
                    serde_json::from_str(&json_details).json(json_details)?
                } else {
                    let new: Result<forge::JsonInstallProfile, serde_json::Error> =
                        serde_json::from_str(&json);
                    if let Ok(new) = new {
                        new.versionInfo
                    } else {
                        return Err(err).json(json)?;
                    }
                }
            }
        };
        Ok(json_details)
    }

    pub async fn setup_optifine(
        &self,
        game_arguments: &mut Vec<String>,
    ) -> Result<Option<(JsonOptifine, PathBuf)>, GameLaunchError> {
        if !matches!(self.config.mod_type, Loader::OptiFine) {
            return Ok(None);
        }

        let (optifine_json, jar) = JsonOptifine::read(&self.instance_name).await?;
        if let Some(arguments) = &optifine_json.arguments {
            game_arguments.extend(arguments.game.clone());
        } else if let Some(arguments) = &optifine_json.minecraftArguments {
            let new: Vec<String> = arguments.split(' ').map(str::to_owned).collect();
            *game_arguments = deduplicate_game_args(game_arguments, &new);
        }

        Ok(Some((optifine_json, jar)))
    }

    pub fn fill_java_arguments(&self, java_arguments: &mut Vec<String>) {
        for argument in java_arguments {
            replace_var(
                argument,
                "classpath_separator",
                &CLASSPATH_SEPARATOR.to_string(),
            );
            // I think this argument is only used by forge? Not sure
            replace_var(argument, "library_directory", "../forge/libraries");
            replace_var(argument, "version_name", self.version_json.get_id());
        }
    }

    pub fn setup_logging(&self, java_arguments: &mut Vec<String>) -> Result<(), GameLaunchError> {
        if let Some(logging) = &self.version_json.logging {
            let logging_path = self
                .instance_dir
                .join(format!("logging-{}", logging.client.file.id));
            let logging_path = logging_path
                .to_str()
                .ok_or(GameLaunchError::PathBufToString(logging_path.clone()))?;
            java_arguments.push(format!("-Dlog4j.configurationFile={logging_path}"));
        }
        Ok(())
    }

    pub fn get_main_class(
        &self,
        fabric_json: Option<&FabricJSON>,
        forge_json: Option<&forge::JsonDetails>,
        optifine_json: Option<&(JsonOptifine, PathBuf)>,
    ) -> String {
        if let Some(main_class_override) = self.main_class_override() {
            main_class_override
        } else if let Some(fabric_json) = fabric_json {
            fabric_json.mainClass.clone()
        } else if let Some(forge_json) = forge_json {
            forge_json.mainClass.clone()
        } else if let Some((optifine_json, _)) = &optifine_json {
            optifine_json.mainClass.clone()
        } else {
            self.version_json.mainClass.clone()
        }
    }

    pub async fn get_class_path(
        &self,
        fabric_json: Option<&FabricJSON>,
        forge_json: Option<&forge::JsonDetails>,
        optifine_json: Option<&(JsonOptifine, PathBuf)>,
        main_class: &str,
    ) -> Result<String, GameLaunchError> {
        // `class_path` is the actual classpath argument
        // string that will be passed to Minecraft as a Java argument.
        let mut class_path = String::new();
        // `classpath_entries` is a `HashSet` that's only responsible for
        // detecting and eliminating duplicate entries
        // (because Minecraft doesn't like them).
        let mut classpath_entries = HashSet::new();

        self.classpath_forge_and_neoforge(forge_json, &mut class_path, &mut classpath_entries)
            .await?;
        if optifine_json.is_some() {
            self.classpath_optifine(&mut class_path).await?;
        }
        self.classpath_fabric_and_quilt(fabric_json, &mut class_path, &mut classpath_entries)?;

        // Vanilla libraries, have to load after everything else
        self.classpath_vanilla(&mut class_path, &mut classpath_entries, main_class)
            .await?;

        // Sometimes mod loaders/core mods try to "override" their own
        // version of a library over the base game. This code is set up
        // so that the loaders load the libraries they like, then the game
        // only loads the stuff that hasn't been already loaded.
        //
        // classpath_entries is a HashSet that determines if an overridden
        // version of a library has already been loaded.

        let instance = InstanceSelection::Instance(self.instance_name.clone());
        let jar_path = jarmod::build(&instance).await?;
        debug_assert!(
            jar_path.is_file(),
            "Minecraft JAR file should exist\nPath: {jar_path:?}"
        );
        let jar_path = jar_path
            .to_str()
            .ok_or(GameLaunchError::PathBufToString(jar_path.clone()))?;
        class_path.push_str(jar_path);

        Ok(class_path)
    }

    fn main_class_override(&self) -> Option<String> {
        let forced_main_class = if self.version_json.is_before_or_eq(V_PRECLASSIC_LAST) {
            "com.mojang.minecraft.RubyDung"
        } else if self.version_json.is_before_or_eq(V_1_5_2) {
            "net.minecraft.launchwrapper.Launch"
        } else {
            "net.minecraft.client.main.Main"
        };
        self.config
            .custom_jar
            .as_ref()
            .is_some_and(|n| n.autoset_main_class)
            .then(|| forced_main_class.to_owned())
            .or_else(|| {
                self.config
                    .main_class_override
                    .clone()
                    .filter(|n| !n.is_empty())
            })
    }

    fn classpath_fabric_and_quilt(
        &self,
        fabric_json: Option<&FabricJSON>,
        class_path: &mut String,
        classpath_entries: &mut HashSet<String>,
    ) -> Result<(), GameLaunchError> {
        if let Some(fabric_json) = fabric_json {
            for library in &fabric_json.libraries {
                if !library.is_allowed() {
                    continue;
                }
                if library.is_lwjgl2() && self.version_json.is_before_or_eq(V_1_12_2) {
                    continue;
                }
                if let Some(name) = remove_version_from_library(&library.name) {
                    if self
                        .version_json
                        .q_patch_overrides
                        .contains(&name.replace(':', "."))
                    {
                        continue;
                    }
                    if !classpath_entries.insert(name) {
                        continue;
                    }
                }

                let library_path = self.instance_dir.join("libraries").join(library.get_path());
                debug_assert!(
                    library_path.is_file(),
                    "Couldn't find library {library_path:?}"
                );
                class_path.push_str(
                    library_path
                        .to_str()
                        .ok_or(GameLaunchError::PathBufToString(library_path.clone()))?,
                );
                class_path.push(CLASSPATH_SEPARATOR);
            }
        }
        Ok(())
    }

    async fn classpath_optifine(&self, class_path: &mut String) -> Result<(), GameLaunchError> {
        let jar_file_location = self.instance_dir.join(".minecraft/libraries");
        let jar_files = find_jar_files(&jar_file_location).await?;
        for jar_file in jar_files {
            debug_assert!(jar_file.is_file());
            class_path.push_str(
                jar_file
                    .to_str()
                    .ok_or(GameLaunchError::PathBufToString(jar_file.clone()))?,
            );
            class_path.push(CLASSPATH_SEPARATOR);
        }
        Ok(())
    }

    async fn classpath_forge_and_neoforge(
        &self,
        forge_json: Option<&forge::JsonDetails>,
        class_path: &mut String,
        classpath_entries: &mut HashSet<String>,
    ) -> Result<(), GameLaunchError> {
        let Some(forge_json) = forge_json else {
            return Ok(());
        };

        let classpath_path = self.instance_dir.join("forge/classpath.txt");
        let forge_classpath = tokio::fs::read_to_string(&classpath_path)
            .await
            .path(classpath_path)?;

        let mut new_classpath = forge_classpath.clone();

        // WTF: This is horrible but necessary
        //
        // When launching Minecraft 1.21.5 NeoForge,
        // Java canonicalizes the module path ("-p path/to/something.jar")
        // and then it complains that the canonicalized and relative
        // paths are not the same. It is not smart enough to figure that shit
        // out.
        //
        // So I have to remove all the libraries from the classpath which
        // are in the module path.
        if let Some(args) = &forge_json.arguments {
            if let Some(jvm) = &args.jvm {
                if let Some(module_path) = get_after_p(jvm) {
                    for lib in module_path
                        .replace("${library_directory}", "../forge/libraries")
                        .replace("${classpath_separator}", &CLASSPATH_SEPARATOR.to_string())
                        .split(CLASSPATH_SEPARATOR)
                    {
                        if let Some(n) =
                            remove_substring(&new_classpath, &format!("{lib}{CLASSPATH_SEPARATOR}"))
                        {
                            new_classpath = n;
                        }
                    }
                }
            }
        }

        class_path.push_str(&new_classpath);

        let classpath_entries_path = self.instance_dir.join("forge/clean_classpath.txt");
        if let Ok(forge_classpath_entries) =
            tokio::fs::read_to_string(&classpath_entries_path).await
        {
            for entry in forge_classpath_entries.lines() {
                classpath_entries.insert(entry.to_owned());
            }
        } else {
            self.migrate_create_forge_clean_classpath(
                forge_classpath,
                classpath_entries,
                classpath_entries_path,
            )
            .await?;
        }

        Ok(())
    }

    async fn classpath_vanilla(
        &self,
        class_path: &mut String,
        classpath_entries: &mut HashSet<String>,
        main_class: &str,
    ) -> Result<(), GameLaunchError> {
        let downloader = GameDownloader::with_existing_instance(
            self.version_json.clone(),
            self.instance_dir.clone(),
            None,
        );

        for library in self
            .version_json
            .libraries
            .iter()
            .filter(|n| n.is_allowed())
        {
            self.add_entry_to_classpath(
                classpath_entries,
                class_path,
                &downloader,
                library,
                main_class,
            )
            .await?;
        }
        Ok(())
    }

    async fn add_entry_to_classpath(
        &self,
        classpath_entries: &mut HashSet<String>,
        class_path: &mut String,
        downloader: &GameDownloader,
        library: &Library,
        main_class: &str,
    ) -> Result<(), GameLaunchError> {
        if let Some(name) = library
            .name
            .as_ref()
            .and_then(|name| remove_version_from_library(name))
        {
            if classpath_entries.contains(&name) {
                return Ok(());
            }
            classpath_entries.insert(name);
        }
        let Some(artifact) = library.get_artifact() else {
            return Ok(());
        };

        // Check for LWJGL override
        let library_path =
            if let Some(override_path) = self.get_lwjgl_override_path(library, &artifact).await? {
                override_path
            } else {
                self.instance_dir
                    .join("libraries")
                    .join(artifact.get_path())
            };

        if !library_path.exists() {
            pt!("library {library_path:?} not found! Downloading...");
            if let Err(err) = downloader.download_library(library, Some(&artifact)).await {
                err!("Couldn't download library! Skipping...\n{err}");
            } else if !library_path.exists() {
                err!("Library still doesn't exist... failed?");
            }
        }
        #[allow(unused_mut)]
        let Some(mut library_path) = library_path.to_str() else {
            return Err(GameLaunchError::PathBufToString(library_path));
        };
        if main_class != "org.mcphackers.launchwrapper.Launch" && library_path.contains("20230311")
        {
            return Ok(());
        }
        if library_path.contains("paulscode")
            && !self.version_json.is_before_or_eq(V_PAULSCODE_LAST)
        {
            // Minecraft stopped using paulscode since 1.14
            // but BetterJSONs still includes it as a dependency,
            // leading to some class conflicts.
            return Ok(());
        }

        #[cfg(target_os = "windows")]
        if library_path.starts_with(r"\\?\") {
            library_path = &library_path[4..];
        }

        class_path.push_str(library_path);
        class_path.push(CLASSPATH_SEPARATOR);
        Ok(())
    }

    /// Check if a library needs LWJGL override and return the new path if so
    async fn get_lwjgl_override_path(
        &self,
        library: &Library,
        _artifact: &ql_core::json::version::LibraryDownloadArtifact,
    ) -> Result<Option<PathBuf>, GameLaunchError> {
        // Check if LWJGL override is configured
        let Some(lwjgl_version) = &self.config_json.lwjgl_version else {
            return Ok(None);
        };

        // Check if this library is an LWJGL library
        let Some(name) = &library.name else {
            return Ok(None);
        };

        if !name.starts_with("org.lwjgl") {
            return Ok(None);
        }

        // Parse library name: "org.lwjgl:lwjgl-opengl:3.2.3" or "org.lwjgl.lwjgl:lwjgl:2.9.4"
        let parts: Vec<&str> = name.split(':').collect();
        if parts.len() < 3 {
            return Ok(None);
        }

        let (_group_id, artifact_id, _old_version) = (parts[0], parts[1], parts[2]);
        let classifier = if parts.len() >= 4 {
            Some(parts[3])
        } else {
            None
        };

        // Note: we intentionally allow LWJGL 2.x/3.x mismatch overrides.
        // The UI warns on apply; users may still force it for testing.

        // Build new library path with custom LWJGL version
        let new_path = lwjgl::build_lwjgl_library_path(lwjgl_version, artifact_id, classifier);
        let library_path = self.instance_dir.join("libraries").join(&new_path);

        // Download if doesn't exist
        if !library_path.exists() {
            pt!("Downloading custom LWJGL library: {}", artifact_id);
            self.download_custom_lwjgl_library(
                lwjgl_version,
                artifact_id,
                classifier,
                &library_path,
            )
            .await?;
        }

        Ok(Some(library_path))
    }

    /// Download a custom LWJGL library from Maven Central
    async fn download_custom_lwjgl_library(
        &self,
        version: &str,
        module: &str,
        classifier: Option<&str>,
        dest_path: &Path,
    ) -> Result<(), GameLaunchError> {
        // Skip lwjgl-freetype for versions before 3.3.2 (it didn't exist)
        if module == "lwjgl-freetype" && lwjgl::is_before_332(version) {
            return Ok(());
        }

        let url = lwjgl::build_lwjgl_maven_url(version, module, classifier);

        // Create parent directories
        if let Some(parent) = dest_path.parent() {
            tokio::fs::create_dir_all(parent).await.path(parent)?;
        }

        // Download the library (streamed to disk) with progress
        let label = format!(
            "Downloading LWJGL {version} {}{}",
            module,
            classifier.map(|c| format!(" ({c})")).unwrap_or_default()
        );
        file_utils::download_file_to_path_with_progress(
            &url,
            false,
            dest_path,
            self.launch_progress_sender.as_ref(),
            Some(&label),
        )
        .await?;

        // If this is a natives JAR, extract it
        if classifier.is_some() && classifier.unwrap().starts_with("natives-") {
            let natives_dir = self.instance_dir.join("libraries/natives");
            tokio::fs::create_dir_all(&natives_dir)
                .await
                .path(&natives_dir)?;

            pt!("Extracting LWJGL natives: {}", module);

            if let Some(sender) = &self.launch_progress_sender {
                let _ = sender.send(GenericProgress {
                    done: 1,
                    total: 1,
                    message: Some(format!("Extracting LWJGL natives: {module}")),
                    has_finished: false,
                });
            }

            let file = std::fs::File::open(dest_path).path(dest_path)?;
            if let Err(e) = file_utils::extract_zip_archive(file, &natives_dir, true).await {
                err!("Failed to extract LWJGL natives: {}", e);
                return Err(IoError::Io {
                    error: format!("Failed to extract LWJGL natives: {e}"),
                    path: natives_dir.clone(),
                }
                .into());
            }

            // Mark the natives directory with the new LWJGL version
            let version_file = natives_dir.join(".lwjgl_version");
            tokio::fs::write(&version_file, version)
                .await
                .path(version_file)?;
        }

        if let Some(sender) = &self.launch_progress_sender {
            let _ = sender.send(GenericProgress {
                done: 1,
                total: 1,
                message: Some("LWJGL download complete".to_owned()),
                has_finished: true,
            });
        }

        Ok(())
    }

    pub async fn get_java_command(&mut self) -> Result<(Command, PathBuf), GameLaunchError> {
        if let Some(java_override) = self.config.get_java_override() {
            info!("Java (override): {java_override:?}\n");
            return Ok((Command::new(&java_override), java_override));
        }
        let version = if let Some(version) = self.config.java_override_version {
            version.into()
        } else if let Some(version) = self.version_json.javaVersion.clone() {
            version.into()
        } else {
            JavaVersion::Java8
        };

        let program = get_java_binary(
            version,
            if cfg!(target_os = "windows") && self.config.enable_logger.unwrap_or(true) {
                "javaw"
            } else {
                "java"
            },
            self.java_install_progress_sender.take().as_ref(),
        )
        .await?;
        info!("Java: {program:?}\n");
        Ok((Command::new(&program), program))
    }

    pub async fn cleanup_junk_files(&self) -> Result<(), GameLaunchError> {
        let forge_dir = self.instance_dir.join("forge");

        if forge_dir.exists() {
            delete_junk_file(&forge_dir, "ClientInstaller.class").await?;
            delete_junk_file(&forge_dir, "ClientInstaller.java").await?;
            delete_junk_file(&forge_dir, "ForgeInstaller.class").await?;
            delete_junk_file(&forge_dir, "ForgeInstaller.java").await?;
            delete_junk_file(&forge_dir, "launcher_profiles.json").await?;
            delete_junk_file(&forge_dir, "launcher_profiles_microsoft_store.json").await?;

            let versions_dir = forge_dir.join("versions");
            delete_junk_dir(&versions_dir.join(self.version_json.get_id())).await?;
            delete_junk_dir(&versions_dir.join(&self.version_json.id)).await?;
        }

        Ok(())
    }

    pub async fn get_command(
        &mut self,
        game_arguments: Vec<String>,
        java_arguments: Vec<String>,
    ) -> Result<(Command, PathBuf), GameLaunchError> {
        let (mut command, mut path) = self.get_java_command().await?;

        let prefix_commands = self.config.build_launch_prefix(
            self.global_settings
                .as_ref()
                .and_then(|n| n.pre_launch_prefix.as_deref())
                .unwrap_or_default(),
        );
        if prefix_commands.is_empty() {
            // No prefix, use normal Java command
            command.args(
                java_arguments
                    .iter()
                    .chain(game_arguments.iter())
                    .filter(|n| !n.is_empty()),
            );
        } else {
            info!("Prefix: {prefix_commands:?}");

            let original_java_path = path.to_string_lossy().to_string();
            let mut new_command = Command::new(&prefix_commands[0]);

            if prefix_commands.len() > 1 {
                new_command.args(&prefix_commands[1..]);
            }
            new_command.arg(original_java_path);
            new_command.args(
                java_arguments
                    .iter()
                    .chain(game_arguments.iter())
                    .filter(|n| !n.is_empty()),
            );

            command = new_command;
            path = PathBuf::from(&prefix_commands[0]);
        }

        command.current_dir(&self.minecraft_dir);
        if self.config.enable_logger.unwrap_or(true) {
            command.stdout(Stdio::piped()).stderr(Stdio::piped());
        }

        #[cfg(all(target_arch = "aarch64", target_os = "linux"))]
        {
            // Minecraft 21w19a release date (1.17 snapshot)
            // Not sure if this is the right place to start,
            // but the env var started being required sometime between 1.16.5 and 1.17
            const MC_1_17: &str = "2021-05-12T11:19:15+00:00";

            if let (Ok(dt), Ok(v1_17)) = (
                chrono::DateTime::parse_from_rfc3339(&self.version_json.releaseTime),
                chrono::DateTime::parse_from_rfc3339(MC_1_17),
            ) {
                // On Raspberry Pi (aarch64 linux), the game crashes with some GL
                // error. Adding this environment variable fixes it.
                if dt >= v1_17 {
                    command.env("MESA_GL_VERSION_OVERRIDE", "3.3");
                }
                // I don't know if this is the perfect solution,
                // contact me if there's a better way
            }
        }
        Ok((command, path))
    }
}

async fn get_instance_dir(instance_name: &str) -> Result<PathBuf, GameLaunchError> {
    if instance_name.is_empty() {
        return Err(GameLaunchError::InstanceNotFound);
    }

    let launcher_dir = &*LAUNCHER_DIR;
    tokio::fs::create_dir_all(&launcher_dir)
        .await
        .path(launcher_dir)?;

    let instances_folder_dir = launcher_dir.join("instances");
    tokio::fs::create_dir_all(&instances_folder_dir)
        .await
        .path(&instances_folder_dir)?;

    let instance_dir = instances_folder_dir.join(instance_name);
    if !instance_dir.exists() {
        return Err(GameLaunchError::InstanceNotFound);
    }
    Ok(instance_dir)
}

async fn delete_junk_file(forge_dir: &Path, path: &str) -> Result<(), GameLaunchError> {
    let path = forge_dir.join(path);
    if path.exists() {
        tokio::fs::remove_file(&path).await.path(path)?;
    }
    Ok(())
}

async fn delete_junk_dir(dir: &Path) -> Result<(), GameLaunchError> {
    if dir.is_dir() {
        tokio::fs::remove_dir_all(&dir).await.path(dir)?;
    }
    Ok(())
}

fn remove_version_from_library(library: &str) -> Option<String> {
    // Split the input string by colons
    let parts: Vec<&str> = library.split(':').collect();

    // Ensure the input has exactly three parts (group, name, version)
    if parts.len() == 3 {
        // Return the first two parts joined by a colon
        Some(format!("{}:{}", parts[0], parts[1]))
    } else {
        // Return None if the input format is incorrect
        None
    }
}

async fn find_jar_files(dir_path: &Path) -> Result<Vec<PathBuf>, IoError> {
    let mut jar_files = Vec::new();

    let mut dir = tokio::fs::read_dir(dir_path).await.path(dir_path)?;
    // Recursively traverse the directory
    while let Ok(Some(entry)) = dir.next_entry().await {
        let path = entry.path();

        if path.is_dir() {
            // If the entry is a directory, recursively search it
            jar_files.extend(Box::pin(find_jar_files(&path)).await?);
        } else if let Some(extension) = path.extension() {
            // If the entry is a file, check if it has a .jar extension
            if extension == "jar" {
                jar_files.push(path);
            }
        }
    }

    Ok(jar_files)
}

/// Moves the game assets from the old path:
///
/// `QuantumLauncher/instances/INSTANCE_NAME/assets/`
///
/// to the usual one:
///
/// `QuantumLauncher/assets/ASSETS_NAME/`
///
/// Old versions of the launcher put the assets at the
/// old path. This migrates it to the new path.
///
/// This applies to early development builds of the
/// launcher (before v0.1), most people won't ever
/// need to run this aside from the early beta testers.
async fn migrate_to_new_assets_path(
    old_assets_path: &Path,
    assets_path: &Path,
) -> Result<(), IoError> {
    info!("Migrating old assets to new path...");
    file_utils::copy_dir_recursive(old_assets_path, assets_path).await?;
    tokio::fs::remove_dir_all(old_assets_path)
        .await
        .path(old_assets_path)?;
    info!("Finished");
    Ok(())
}

fn get_after_p(args: &[String]) -> Option<String> {
    args.iter()
        .position(|arg| arg == "-p")
        .and_then(|index| args.get(index + 1))
        .cloned()
}

/// Removes substring
///
/// `"hello", "ell" -> "ho"`
fn remove_substring(original: &str, to_remove: &str) -> Option<String> {
    if let Some(pos) = original.find(to_remove) {
        let mut result = String::with_capacity(original.len() - to_remove.len());
        result.push_str(&original[..pos]);
        result.push_str(&original[pos + to_remove.len()..]);
        Some(result)
    } else {
        None
    }
}

fn deduplicate_game_args(arr1: &[String], arr2: &[String]) -> Vec<String> {
    // Helper function to insert key-value pairs in order
    fn insert_pairs(arr: &[String], result: &mut Vec<String>, seen_keys: &mut HashSet<String>) {
        let arr: Vec<String> = arr.iter().filter(|n| !n.is_empty()).cloned().collect();
        for i in (0..arr.len()).step_by(2) {
            let key = arr[i].clone();
            let value = arr.get(i + 1).cloned();
            if seen_keys.contains(&key) {
                if let Some(value) = value {
                    // Update value if the key already exists in result (i.e., in case of conflict, overwrite)
                    if let Some(pos) = result.iter().position(|x| x == &key) {
                        if let Some(spot) = result.get_mut(pos + 1) {
                            *spot = value; // Update the value for this key
                        }
                    }
                }
            } else {
                result.push(key.clone());
                if let Some(value) = value {
                    result.push(value.clone());
                }
                seen_keys.insert(key);
            }
        }
    }

    let mut result = Vec::new();
    let mut seen_keys = HashSet::new();

    insert_pairs(arr1, &mut result, &mut seen_keys);
    // Second array overwrites first
    insert_pairs(arr2, &mut result, &mut seen_keys);

    // HashMap -> Vec<String> (key, value, key, value, ...)
    result
}
