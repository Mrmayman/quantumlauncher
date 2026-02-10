use owo_colors::{OwoColorize, Style};
use ql_core::{
    eeprintln, err, info,
    json::{InstanceConfigJson, VersionDetails},
    InstanceSelection, IntoStringError, ListEntry, Loader, OptifineUniqueVersion, LAUNCHER_DIR,
};
use ql_instances::auth::{self, AccountType};
use ql_mod_manager::loaders::LoaderInstallResult;
use std::{path::PathBuf, process::exit};

use crate::{
    cli::{helpers::render_row, QLoader},
    config::LauncherConfig,
    state::get_entries,
};

use super::PrintCmd;

pub fn list_available_versions() {
    use std::io::Write;

    eeprintln!("Listing downloadable versions...");
    let (versions, _) = match tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(ql_instances::list_versions())
        .strerr()
    {
        Ok(n) => n,
        Err(err) => {
            panic!("Could not list versions!\n{err}");
        }
    };

    let mut stdout = std::io::stdout().lock();
    for version in versions {
        writeln!(stdout, "{version}").unwrap();
    }
}

pub fn list_instances(
    properties: Option<&[String]>,
    is_server: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::fmt::Write;

    let mut cmds: Vec<PrintCmd> = properties
        .unwrap_or_default()
        .iter()
        .filter_map(|n| match n.as_str() {
            "name" => Some(PrintCmd::Name),
            "version" => Some(PrintCmd::Version),
            "loader" => Some(PrintCmd::Loader),
            _ => None,
        })
        .collect();
    if cmds.is_empty() {
        cmds.push(PrintCmd::Name);
    }

    let runtime = tokio::runtime::Runtime::new()?;

    let dirname = if is_server { "servers" } else { "instances" };
    let (instances, _) = tokio::runtime::Runtime::new()?.block_on(get_entries(is_server))?;

    let mut cmds_name = String::new();
    let mut cmds_version = String::new();
    let mut cmds_loader = String::new();

    for instance in instances {
        let instance_dir = LAUNCHER_DIR.join(dirname).join(&instance);
        for cmd in &cmds {
            match cmd {
                PrintCmd::Name => {
                    _ = writeln!(cmds_name, "{}", instance.bold().underline());
                }
                PrintCmd::Version => {
                    match runtime.block_on(VersionDetails::load_from_path(&instance_dir)) {
                        Ok(json) => {
                            cmds_version.push_str(&json.id);
                        }
                        Err(err) => {
                            err!("{err}");
                        }
                    }
                    cmds_version.push('\n');
                }
                PrintCmd::Loader => {
                    let config_json =
                        match runtime.block_on(InstanceConfigJson::read_from_dir(&instance_dir)) {
                            Ok(json) => json,
                            Err(err) => {
                                err!("{err}");
                                cmds_loader.push('\n');
                                continue;
                            }
                        };
                    let m = config_json.mod_type;

                    match m {
                        Loader::Vanilla => writeln!(cmds_loader, "{}", m.bright_black()),
                        Loader::Fabric => writeln!(cmds_loader, "{}", m.bright_green()),
                        Loader::Quilt => writeln!(cmds_loader, "{}", m.bright_purple()),
                        Loader::Forge => writeln!(cmds_loader, "{}", m.bright_yellow()),
                        Loader::Neoforge => writeln!(cmds_loader, "{}", m.yellow()),
                        Loader::OptiFine => writeln!(cmds_loader, "{}", m.red().bold()),
                        Loader::Paper => writeln!(cmds_loader, "{}", m.blue()),
                        Loader::Liteloader => writeln!(cmds_loader, "{}", m.bright_blue()),
                        Loader::Modloader => writeln!(cmds_loader, "{m}"),
                        Loader::Rift => writeln!(cmds_loader, "{}", m.bold().underline()),
                    }?;
                }
            }
        }
    }

    let Some((terminal_size::Width(width), _)) = terminal_size::terminal_size() else {
        println!("{cmds_name}\n\n{cmds_loader}\n\n{cmds_version}");
        return Ok(());
    };

    let cmds: Vec<(String, Option<Style>)> = cmds
        .iter()
        .map(|n| match n {
            PrintCmd::Name => (cmds_name.clone(), None),
            PrintCmd::Version => (cmds_version.clone(), None),
            PrintCmd::Loader => (cmds_loader.clone(), None),
        })
        .collect();

    println!("{}", render_row(width, &cmds, true).unwrap());

    Ok(())
}

pub async fn create_instance(
    instance_name: String,
    version: String,
    skip_assets: bool,
    servers: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let entry = ListEntry::new(version);
    if servers {
        ql_servers::create_server(instance_name, entry, None).await?;
    } else {
        ql_instances::create_instance(instance_name, entry, None, !skip_assets).await?;
    }

    Ok(())
}

pub fn delete_instance(
    instance_name: String,
    force: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if !force {
        println!(
            "{} {instance_name}?",
            "Are you SURE you want to delete the instance"
                .yellow()
                .bold()
        );
        println!("This can't be undone, you will lose all your data");
        if !confirm_action() {
            println!("Cancelled");
            return Ok(());
        }
    }

    let selected_instance = InstanceSelection::Instance(instance_name.clone());
    let deleted_instance_dir = selected_instance.get_instance_path();
    std::fs::remove_dir_all(&deleted_instance_dir)?;
    info!("Deleted instance {instance_name}");

    Ok(())
}

fn confirm_action() -> bool {
    use std::io::Write;

    print!("[Y/n] ");
    std::io::stdout().flush().unwrap();

    let mut user_input = String::new();
    std::io::stdin().read_line(&mut user_input).unwrap();

    let user_input = user_input.trim().to_lowercase();

    let res = match user_input.as_str() {
        "y" | "yes" | "" => true,
        "n" | "no" => false,
        _ => {
            println!("\nInvalid input. Please respond with 'Y' or 'n'.\n");
            confirm_action() // Retry
        }
    };
    println!();
    res
}

pub async fn launch_instance(
    instance_name: String,
    username: String,
    use_account: bool,
    servers: bool,
    show_progress: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let account = refresh_account(&username, use_account, show_progress).await?;

    let child = if servers {
        // TODO: stdin input
        ql_servers::run(instance_name.clone(), None).await?
    } else {
        ql_instances::launch(
            instance_name.clone(),
            username,
            None,
            account.clone(),
            None, // No global defaults in CLI mode
            Vec::new(),
        )
        .await?
    };

    let mut censors = Vec::new();
    if let Some(token) = account.as_ref().and_then(|n| n.access_token.as_ref()) {
        censors.push(token.clone());
    }

    match child.read_logs(censors, None).await {
        Some(Ok((s, _, diag))) => {
            info!("Game exited with code {s}");
            if let Some(diag) = diag {
                err!("{diag}");
            }
            exit(s.code().unwrap_or_default());
        }
        Some(Err(err)) => Err(err)?,
        None => {}
    }
    Ok(())
}

async fn refresh_account(
    username: &String,
    use_account: bool,
    show_progress: bool,
) -> Result<Option<auth::AccountData>, Box<dyn std::error::Error>> {
    Ok(if use_account {
        let config = LauncherConfig::load_s()?;
        let Some((real_name, account)) = config.accounts.as_ref().and_then(|accounts| {
            accounts.get_key_value(username).or_else(|| {
                accounts
                    .iter()
                    .find(|n| n.1.username_nice.as_ref().is_some_and(|n| n == username))
            })
        }) else {
            err!("No logged-in account called {username:?} was found!");
            exit(1);
        };

        if show_progress {
            tokio::task::spawn_blocking(|| {
                notify_rust::Notification::new()
                    .summary("Launching game")
                    .body("Refreshing account...")
                    .show()
            });
        };

        match account.account_type.as_deref() {
            // Hook: Account types
            Some(kind @ ("ElyBy" | "LittleSkin")) => {
                let account_type = if kind == "ElyBy" {
                    AccountType::ElyBy
                } else {
                    AccountType::LittleSkin
                };
                let refresh_token = auth::read_refresh_token(real_name, account_type)?;
                Some(
                    auth::yggdrasil::login_refresh(
                        real_name.to_owned(),
                        refresh_token,
                        account_type,
                    )
                    .await?,
                )
            }
            _ => {
                let refresh_token = auth::read_refresh_token(real_name, AccountType::Microsoft)?;
                Some(auth::ms::login_refresh(real_name.clone(), refresh_token, None).await?)
            }
        }
    } else {
        None
    })
}

pub async fn loader(cmd: QLoader, servers: bool) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        QLoader::Info { instance } => {
            let json =
                InstanceConfigJson::read(&InstanceSelection::new(&instance, servers)).await?;
            println!("Kind: {}", json.mod_type);
            if let Some(info) = json.mod_type_info {
                if let Some(version) = info.version {
                    println!("Version: {version}");
                }
                if let Some(backend) = info.backend_implementation {
                    println!("Backend: {backend}");
                }
                if let Some(jar) = info.optifine_jar {
                    println!("OptiFine Installation: {jar}");
                }
            }
        }
        QLoader::Install {
            instance,
            loader,
            more,
            version,
        } => {
            if loader.eq_ignore_ascii_case("vanilla") {
                err!("Vanilla refers to the base game.\n    Maybe you meant `./quantum_launcher loader uninstall ...`");
                exit(1);
            }
            let Some(loader) = Loader::ALL
                .iter()
                .copied()
                .find(|n| n.to_modrinth_str().eq_ignore_ascii_case(&loader))
            else {
                err!("Invalid loader: {loader}");
                exit(1)
            };

            let instance = InstanceSelection::new(&instance, servers);
            let mt = InstanceConfigJson::read(&instance).await?.mod_type;

            if mt == loader {
                err!("{mt} is already installed!");
                exit(0);
            }
            if !(mt == Loader::Vanilla
                || (mt == Loader::Forge && loader == Loader::OptiFine)
                || (mt == Loader::OptiFine && loader == Loader::Forge))
            {
                err!(
                    r"You can't install a loader on top of another loader!
    Did you mean to uninstall the other one first: `./quantum_launcher loader uninstall ...`"
                );
                exit(1);
            }

            match ql_mod_manager::loaders::install_specified_loader(
                instance.clone(),
                loader,
                None,
                version,
            )
            .await?
            {
                LoaderInstallResult::Ok => {}
                LoaderInstallResult::NeedsOptifine => {
                    install_optifine(more, instance).await?;
                }
                LoaderInstallResult::Unsupported => {
                    err!("This loader is unsupported!");
                    exit(1);
                }
            }
        }
        QLoader::Uninstall { instance } => {
            let instance = InstanceSelection::new(&instance, servers);
            ql_mod_manager::loaders::uninstall_loader(instance).await?;
        }
    }
    Ok(())
}

async fn install_optifine(
    more: Option<String>,
    instance: InstanceSelection,
) -> Result<(), Box<dyn std::error::Error + 'static>> {
    let details = VersionDetails::load(&instance).await?;
    if details.get_id() == "b1.7.3" {
        ql_mod_manager::loaders::optifine::install_b173(
            instance,
            OptifineUniqueVersion::B1_7_3.get_url().0,
        )
        .await?;
        return Ok(());
    }

    let Some(more) = more else {
        err!(
            r"Please download the OptiFine installer at: https://optifine.net/downloads
    and pass the path via: `quantum_launcher loader install optifine path/to/installer.jar`"
        );
        exit(1);
    };

    ql_mod_manager::loaders::optifine::install(
        instance.get_name().to_owned(),
        PathBuf::from(more),
        None,
        None,
        OptifineUniqueVersion::from_version(details.get_id()),
    )
    .await?;
    Ok(())
}
