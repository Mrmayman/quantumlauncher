use clap::{Arg, ArgAction, Command};
use owo_colors::OwoColorize;
use ql_core::{err, LAUNCHER_VERSION_NAME};
use std::io::Write;

use crate::menu_renderer::{DISCORD, GITHUB};

mod command;

fn command() -> Command {
    Command::new(if cfg!(target_os = "windows") {
        ".\\quantum_launcher.exe"
    } else {
        "./quantum_launcher"
    })
    .arg_required_else_help(false)
    .author("Mrmayman")
    .version(LAUNCHER_VERSION_NAME)
    .long_about(long_about())
    .arg(
        Arg::new("tui")
            .long("tui")
            .help("Launch in Terminal User Interface (TUI) mode")
            .action(ArgAction::SetTrue)
    )
    .subcommand(Command::new("create")
        .args([
            Arg::new("instance_name").help("The name of the instance to create").required(true),
            Arg::new("version").help("The version of Minecraft to download").required(true),
            Arg::new("--skip-assets")
                .short('s')
                .long("skip-assets")
                .required(false)
                .help("Skips downloading game assets (sound/music) to speed up downloads")
                .action(ArgAction::SetTrue),
        ])
        .about("Creates a new installation (instance) of Minecraft")
    )
    .subcommand(get_launch_subcommand())
    .subcommand(
        get_list_instance_subcommands("list-instances")
            .short_flag('l')
            .about("Lists all installed Minecraft instances")
            .long_about("Lists all installed Minecraft instances. Can be paired with hyphen-separated-flags like name-loader, name-version, loader-name-version"),
    )
    .subcommand(
        get_list_instance_subcommands("list-servers")
            .about("Lists all installed Minecraft servers")
            .long_about("Lists all installed Minecraft servers. Can be paired with hyphen-separated-flags like name-loader, name-version, loader-name-version")
            .hide(true),
    )
    .subcommand(Command::new("delete")
        .args([
            Arg::new("instance_name").help("The name of the instance to delete").required(true),
            Arg::new("--force")
                .short('f')
                .long("force")
                .required(false)
                .help("Forces deletion without confirmation. DANGEROUS")
                .action(ArgAction::SetTrue),
        ])
        .about("Deletes an instance of Minecraft")
    )
    .subcommand(Command::new("list-available-versions").short_flag('a').about("Lists all downloadable versions, downloading a list from Mojang/Omniarchive"))
    .subcommand(Command::new("--no-sandbox").hide(true)) // This one doesn't do anything, but on Windows i686 it's automatically passed?
}

fn get_launch_subcommand() -> Command {
    Command::new("launch")
        .about("Launches the specified instance")
        .arg_required_else_help(true)
        .args([
            Arg::new("instance_name")
                .help("The name of the instance to launch")
                .required(true),
            Arg::new("username")
                .help("Username of the player")
                .required(true),
            Arg::new("--use-account")
                .short('a')
                .long("use-account")
                .help("Whether to use a logged in account of the given username (if any)")
                .required(false)
                .action(ArgAction::SetTrue),
        ])
}

fn get_list_instance_subcommands(name: &'static str) -> Command {
    Command::new(name)
        // May god forgive me for what I'm about to do
        .subcommand(Command::new("name"))
        .subcommand(Command::new("version"))
        .subcommand(Command::new("loader"))
        .subcommand(Command::new("name-version"))
        .subcommand(Command::new("name-loader"))
        .subcommand(Command::new("version-name"))
        .subcommand(Command::new("version-loader"))
        .subcommand(Command::new("loader-name"))
        .subcommand(Command::new("loader-version"))
        .subcommand(Command::new("name-version-loader"))
        .subcommand(Command::new("name-loader-version"))
        .subcommand(Command::new("version-name-loader"))
        .subcommand(Command::new("version-loader-name"))
        .subcommand(Command::new("loader-name-version"))
        .subcommand(Command::new("loader-version-name"))
}

fn long_about() -> String {
    format!(
        r"
QuantumLauncher: A simple, powerful Minecraft launcher

Usage:
  quantum_launcher                    Launch GUI mode
  quantum_launcher --tui              Launch TUI (Terminal) mode
  quantum_launcher <SUBCOMMAND>       Use CLI commands

Website: https://mrmayman.github.io/quantumlauncher
Github : {GITHUB}
Discord: {DISCORD}"
    )
}

enum PrintCmd {
    Name,
    Version,
    Loader,
}

/// Prints the "intro" to the screen
/// consisting of the **ASCII art logo**, as well as
/// **stylised text saying `QuantumLauncher`**
///
/// The actual data is `include_str!()`ed from
/// - `assets/ascii/icon.txt` for the ASCII art
/// - `assets/ascii/text.txt` for the text logo
///
/// The other files in `assets/ascii` are unused.
fn print_intro() {
    /// Helper function to pad lines to a fixed width
    fn pad_line(line: Option<&str>, width: usize) -> String {
        let line = line.unwrap_or_default();
        if line.len() < width {
            format!("{line:<width$}")
        } else {
            line.to_owned()
        }
    }

    const TEXT_WIDTH: u16 = 39;

    const LOGO: &str = include_str!("../../../assets/ascii/icon.txt");
    const LOGO_WIDTH: u16 = 30;

    if cfg!(target_os = "windows") {
        return;
    }

    let (text, text_len_old) = get_side_text();

    let logo_len: usize = LOGO.lines().count();

    let Some((terminal_size::Width(width), _)) = terminal_size::terminal_size() else {
        return;
    };

    let mut stdout = std::io::stdout().lock();

    // Ok, this code is uncomfortably ugly but bear with me...
    if width > TEXT_WIDTH + LOGO_WIDTH {
        // Screen large enough for Text and Logo
        // to fit side-by-side
        let lines_len = std::cmp::max(text.lines().count(), LOGO.lines().count());
        for i in 0..lines_len {
            let text_line = pad_line(text.lines().nth(i), TEXT_WIDTH as usize);
            let logo_line = pad_line(LOGO.lines().nth(i), LOGO_WIDTH as usize);
            if i >= logo_len {
                _ = write!(stdout, "{logo_line} ");
            } else {
                _ = write!(stdout, "{} ", logo_line.purple().bold());
            }
            if i >= text_len_old {
                _ = write!(stdout, "{text_line}");
            } else {
                _ = write!(stdout, "{}", text_line.bold());
            }
            _ = writeln!(stdout);
        }
    } else if width >= TEXT_WIDTH {
        // Screen only large enough for
        // Text and Logo to fit one after another
        // vertically
        _ = writeln!(stdout, "{}\n{}", LOGO.purple().bold(), text.bold());
    } else if width >= LOGO_WIDTH {
        // Screen only large enough for Logo,
        // not text
        _ = writeln!(stdout, "{}", LOGO.purple().bold());
    } else {
        // Screen is too tiny
        _ = writeln!(stdout, "Quantum Launcher {LAUNCHER_VERSION_NAME}");
    }
    _ = writeln!(stdout);
}

fn get_side_text() -> (String, usize) {
    let mut text = include_str!("../../../assets/ascii/text.txt").to_owned();
    let text_len_old = text.lines().count();

    let mut message = if cfg!(target_os = "windows") {
        "\n A simple, powerful Minecraft launcher".to_owned()
    } else {
        format!(
            "\n {}",
            "A simple, powerful Minecraft launcher".green().bold(),
        )
    };

    message.push_str("\n This window just shows debug info so\n feel free to ignore it\n\n ");

    let list_of_commands = if cfg!(target_os = "windows") {
        "For a list of commands type 'quantum_launcher.exe --help'".to_owned()
    } else {
        format!(
            "For a list of commands type\n {} {}",
            "./quantum_launcher".yellow().bold(),
            "--help".yellow()
        )
    };
    message.push_str(&list_of_commands);

    text.push_str(&message);

    (text, text_len_old)
}

pub fn start_cli(is_dir_err: bool) {
    let command = command();
    let matches = command.clone().get_matches();

    // Check for TUI flag first
    if matches.get_flag("tui") {
        if is_dir_err {
            std::process::exit(1);
        }
        // Launch TUI mode
        if let Err(err) = crate::tui::run_tui() {
            err!("TUI error: {err}");
            std::process::exit(1);
        }
        std::process::exit(0);
    }

    if let Some(subcommand) = matches.subcommand() {
        if is_dir_err {
            std::process::exit(1);
        }
        match subcommand.0 {
            "list-instances" => {
                let command = get_list_instance_subcommand(subcommand);
                command::list_instances(&command, "instances");
                std::process::exit(0);
            }
            "list-servers" => {
                let command = get_list_instance_subcommand(subcommand);
                command::list_instances(&command, "servers");
                std::process::exit(0);
            }
            "list-available-versions" => {
                command::list_available_versions();
                std::process::exit(0);
            }
            "launch" => quit(command::launch_instance(subcommand)),
            "create" => quit(command::create_instance(subcommand)),
            "delete" => quit(command::delete_instance(subcommand)),
            "--no-sandbox" => {}
            err => panic!("Unimplemented command! {err}"),
        }
    } else {
        print_intro();
    }
}

fn quit(res: Result<(), Box<dyn std::error::Error + 'static>>) {
    std::process::exit(if let Err(err) = res {
        err!("{err}");
        1
    } else {
        0
    });
}

fn get_list_instance_subcommand(subcommand: (&str, &clap::ArgMatches)) -> Vec<PrintCmd> {
    if let Some((cmd, _)) = subcommand.1.subcommand() {
        let mut cmds = Vec::new();
        for cmd in cmd.split('-') {
            match cmd {
                "name" => cmds.push(PrintCmd::Name),
                "version" => cmds.push(PrintCmd::Version),
                "loader" => cmds.push(PrintCmd::Loader),
                invalid => {
                    panic!("Invalid subcommand {invalid}! Use any combination of name, version and loader separated by hyphen '-'");
                }
            }
        }
        cmds
    } else {
        vec![PrintCmd::Name]
    }
}
