use owo_colors::{OwoColorize, Style};
use ql_core::{
    err, info,
    json::{InstanceConfigJson, VersionDetails},
    InstanceSelection, IntoIoError, IntoJsonError, IntoStringError, ListEntry, Loader,
    LAUNCHER_DIR,
};
use ql_instances::auth::{self, AccountType};
use std::process::exit;

use crate::{cli::helpers::render_row, config::LauncherConfig, state::get_entries};

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
    cmds: &[PrintCmd],
    is_server: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::fmt::Write;

    let dirname = if is_server { "servers" } else { "instances" };
    let (instances, _) = tokio::runtime::Runtime::new()?.block_on(get_entries(is_server))?;

    let mut cmds_name = String::new();
    let mut cmds_version = String::new();
    let mut cmds_loader = String::new();

    for instance in instances {
        for cmd in cmds {
            match cmd {
                PrintCmd::Name => {
                    _ = writeln!(cmds_name, "{}", instance.bold().underline());
                }
                PrintCmd::Version => {
                    let instance_dir = LAUNCHER_DIR.join(dirname).join(&instance);

                    let path = instance_dir.join("details.json");
                    let json = std::fs::read_to_string(&path).path(path)?;
                    let mut json: VersionDetails = serde_json::from_str(&json).json(json)?;
                    json.fix();

                    cmds_version.push_str(&json.id);
                    cmds_version.push('\n');
                }
                PrintCmd::Loader => {
                    let instance_dir = LAUNCHER_DIR.join(dirname).join(&instance);
                    let path = instance_dir.join("config.json");
                    let config_json = std::fs::read_to_string(&path).path(path)?;
                    let config_json: InstanceConfigJson =
                        serde_json::from_str(&config_json).json(config_json)?;
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
    subcommand: (&str, &clap::ArgMatches),
) -> Result<(), Box<dyn std::error::Error>> {
    let instance_name: &String = subcommand.1.get_one("instance_name").unwrap();
    let version: &String = subcommand.1.get_one("version").unwrap();
    let skip_assets: bool = *subcommand.1.get_one("--skip-assets").unwrap();

    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(ql_instances::create_instance(
        instance_name.clone(),
        ListEntry {
            name: version.clone(),
            is_classic_server: false,
        },
        None,
        !skip_assets,
    ))?;

    Ok(())
}

pub fn delete_instance(
    subcommand: (&str, &clap::ArgMatches),
) -> Result<(), Box<dyn std::error::Error>> {
    let instance_name: &String = subcommand.1.get_one("instance_name").unwrap();
    let force: bool = *subcommand.1.get_one("--force").unwrap();

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
    subcommand: (&str, &clap::ArgMatches),
) -> Result<(), Box<dyn std::error::Error>> {
    let instance_name: &String = subcommand.1.get_one("instance_name").unwrap();
    let username: &String = subcommand.1.get_one("username").unwrap();
    let use_account: bool = *subcommand.1.get_one("--use-account").unwrap();

    let runtime = tokio::runtime::Runtime::new()?;

    let account = refresh_account(username, use_account, &runtime)?;

    let child = runtime.block_on(ql_instances::launch(
        instance_name.clone(),
        username.clone(),
        None,
        account.clone(),
        // No global defaults in CLI mode
        None,
        Vec::new(),
    ))?;

    if let (Some(stdout), Some(stderr)) = {
        let mut child = child.lock().unwrap();
        (child.stdout.take(), child.stderr.take())
    } {
        let mut censors = Vec::new();
        if let Some(token) = account.as_ref().and_then(|n| n.access_token.as_ref()) {
            censors.push(token.clone());
        }

        match runtime.block_on(ql_instances::read_logs(
            stdout,
            stderr,
            child,
            None,
            instance_name.clone(),
            censors,
        )) {
            Ok((s, _)) => {
                info!("Game exited with code {s}");
                exit(s.code().unwrap_or_default());
            }
            Err(err) => {
                err!("{err}");
                exit(1);
            }
        }
    } else {
        exit(0);
    }
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
