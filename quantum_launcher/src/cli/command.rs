use owo_colors::{OwoColorize, Style};
use ql_core::{
    err, info,
    json::{InstanceConfigJson, VersionDetails},
    InstanceSelection, IntoStringError, ListEntry, Loader, LAUNCHER_DIR,
};
use ql_instances::auth::{self, AccountType};
use std::process::exit;

use crate::{
    cli::{helpers::render_row, QLoader},
    config::LauncherConfig,
    state::get_entries,
};

use super::PrintCmd;

pub fn list_available_versions() {
    use std::io::Write;

    eprintln!("Listing downloadable versions...");
    let versions = match tokio::runtime::Runtime::new()
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

                    match Loader::try_from(m.as_str()) {
                        Ok(l) => {
                            _ = match l {
                                Loader::Fabric => writeln!(cmds_loader, "{}", m.bright_green()),
                                Loader::Quilt => writeln!(cmds_loader, "{}", m.bright_purple()),
                                Loader::Forge => writeln!(cmds_loader, "{}", m.bright_yellow()),
                                Loader::Neoforge => writeln!(cmds_loader, "{}", m.yellow()),
                                Loader::OptiFine => {
                                    writeln!(cmds_loader, "{}", m.red().bold())
                                }
                                Loader::Paper => writeln!(cmds_loader, "{}", m.blue()),
                                Loader::Liteloader => writeln!(cmds_loader, "{}", m.bright_blue()),
                                Loader::Modloader => writeln!(cmds_loader, "{}", m),
                                Loader::Rift => writeln!(cmds_loader, "{}", m.bold().underline()),
                            };
                        }
                        Err(_) => {
                            if m == "Vanilla" {
                                _ = writeln!(cmds_loader, "{}", "Vanilla".bright_black());
                            } else {
                                cmds_loader.push_str(&m);
                                cmds_loader.push('\n');
                            }
                        }
                    }
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

pub fn create_instance(
    instance_name: String,
    version: String,
    skip_assets: bool,
    runtime: &tokio::runtime::Runtime,
    servers: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if servers {
        runtime.block_on(ql_servers::create_server(
            instance_name,
            ListEntry {
                is_classic_server: version.starts_with("c0."),
                name: version,
            },
            None,
        ))?;
    } else {
        runtime.block_on(ql_instances::create_instance(
            instance_name,
            ListEntry {
                name: version.clone(),
                is_classic_server: false,
            },
            None,
            !skip_assets,
        ))?;
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

pub fn launch_instance(
    instance_name: String,
    username: String,
    use_account: bool,
    runtime: &tokio::runtime::Runtime,
    servers: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let account = refresh_account(&username, use_account, runtime)?;

    let child = if servers {
        // TODO: stdin input
        runtime.block_on(ql_servers::run(instance_name.clone(), None))?
    } else {
        runtime.block_on(ql_instances::launch(
            instance_name.clone(),
            username,
            None,
            account.clone(),
            None, // No global defaults in CLI mode
            Vec::new(),
        ))?
    };

    let mut censors = Vec::new();
    if let Some(token) = account.as_ref().and_then(|n| n.access_token.as_ref()) {
        censors.push(token.clone());
    }

    if let Some(f) = child.read_logs(censors, None) {
        match runtime.block_on(f) {
            Ok((s, _)) => {
                info!("Game exited with code {s}");
                exit(s.code().unwrap_or_default());
            }
            Err(err) => Err(err)?,
        }
    }
    Ok(())
}

fn refresh_account(
    username: &String,
    use_account: bool,
    runtime: &tokio::runtime::Runtime,
) -> Result<Option<auth::AccountData>, Box<dyn std::error::Error>> {
    Ok(if use_account {
        let config = LauncherConfig::load_s()?;
        let Some(accounts) = config.accounts else {
            err!("You haven't paired any accounts yet! Use the graphical interface to add some.");
            exit(1);
        };
        let Some((real_name, account)) = accounts.get_key_value(username).or_else(|| {
            accounts
                .iter()
                .find(|n| n.1.username_nice.as_ref().is_some_and(|n| n == username))
        }) else {
            err!("No logged-in account called {username:?} was found!");
            exit(1);
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
                Some(runtime.block_on(auth::yggdrasil::login_refresh(
                    real_name.clone(),
                    refresh_token,
                    account_type,
                ))?)
            }
            _ => {
                let refresh_token = auth::read_refresh_token(real_name, AccountType::Microsoft)?;
                Some(runtime.block_on(auth::ms::login_refresh(
                    real_name.clone(),
                    refresh_token,
                    None,
                ))?)
            }
        }
    } else {
        None
    })
}

pub fn loader(
    cmd: QLoader,
    runtime: &tokio::runtime::Runtime,
    servers: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        QLoader::Info { instance } => {
            let json = runtime.block_on(InstanceConfigJson::read(&InstanceSelection::new(
                &instance, servers,
            )))?;
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
            version,
        } => {
            if loader.eq_ignore_ascii_case("vanilla") {
                err!("Vanilla refers to the base game.\n    Maybe you meant `./quantum_launcher loader uninstall ...`?");
                exit(1);
            }
            let Ok(loader) = Loader::try_from(loader.as_str()) else {
                exit(1)
            };
            runtime.block_on(ql_mod_manager::loaders::install_specified_loader(
                InstanceSelection::new(&instance, servers),
                loader,
                None,
                version,
            ))?;
        }
    }
    Ok(())
}
