use clap::{Arg, ArgAction, Command};
use itertools::Itertools;
use owo_colors::{OwoColorize, Style};
use ql_core::{err, LAUNCHER_VERSION_NAME, WEBSITE};

use crate::{
    cli::helpers::render_row,
    menu_renderer::{DISCORD, GITHUB},
};

mod command;
mod helpers;

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
        get_list_instance_command("list")
            .alias("list-instances")
            .short_flag('l')
            .about("Lists all installed Minecraft instances")
            .long_about("Lists all installed Minecraft instances. Can be paired with hyphen-separated-flags like name-loader, name-version, loader-name-version"),
    )
    .subcommand(
        get_list_instance_command("list-servers")
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

fn get_list_instance_command(name: &'static str) -> Command {
    Command::new(name).arg(
        Arg::new("fields")
            .help("Fields to display (any combination of name, version, loader)")
            .num_args(1..) // accept 1 or more
            .action(ArgAction::Append)
            .value_parser(["name", "version", "loader"]),
    )
}

fn long_about() -> String {
    format!(
        r"
QuantumLauncher: A simple, powerful Minecraft launcher

Website: {WEBSITE}
Github : {GITHUB}
Discord: {DISCORD}"
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    const LOGO: &str = include_str!("../../../assets/ascii/icon.txt");
    const LOGO_WIDTH: u16 = 30;

    let text = get_right_text();

    let Some((terminal_size::Width(width), _)) = terminal_size::terminal_size() else {
        return;
    };

    let draw_contents = &[
        (LOGO.to_owned(), Some(Style::new().purple().bold())),
        (text.clone(), None),
    ];

    // If we got enough space for both side-by-side
    if let Some(res) = render_row(width, draw_contents, false) {
        println!("{res}");
    } else {
        if width >= LOGO_WIDTH {
            // Screen only large enough for Logo, not text
            println!("{}", LOGO.purple().bold());
        }
        println!(
            " {} {}\n",
            "Quantum Launcher".purple().bold(),
            LAUNCHER_VERSION_NAME.purple()
        );
    }
}

fn get_right_text() -> String {
    const TEXT: &str = include_str!("../../../assets/ascii/text.txt");

    let message = format!(
        r"{TEXT}
 {}
 {}
 {}

 For a list of commands type
 {help}",
        "A simple, powerful Minecraft launcher".green().bold(),
        "This window shows debug info;".bright_black(),
        "feel free to ignore it".bright_black(),
        help = "./quantum_launcher --help".yellow()
    );

    message
}

pub fn start_cli(is_dir_err: bool) {
    let command = command();
    let matches = command.clone().get_matches();

    if let Some(subcommand) = matches.subcommand() {
        if is_dir_err {
            std::process::exit(1);
        }
        match subcommand.0 {
            "list" | "list-instances" => {
                let command = get_list_instance_subcommand(&subcommand.1);
                quit(command::list_instances(&command, false));
            }
            "list-servers" => {
                let command = get_list_instance_subcommand(&subcommand.1);
                quit(command::list_instances(&command, true));
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

fn get_list_instance_subcommand(matches: &clap::ArgMatches) -> Vec<PrintCmd> {
    if let Some(values) = matches.get_many::<String>("fields") {
        values
            .map(|val| match val.as_str() {
                "name" => PrintCmd::Name,
                "version" => PrintCmd::Version,
                "loader" => PrintCmd::Loader,
                invalid => panic!(
                    "Invalid field {invalid}! Use any combination of name, version, and loader."
                ),
            })
            .unique()
            .collect()
    } else {
        // Default to showing name if no args passed
        vec![PrintCmd::Name]
    }
}
