use clap::{Parser, Subcommand};
use owo_colors::{OwoColorize, Style};
use ql_core::{err, LAUNCHER_VERSION_NAME, REDACT_SENSITIVE_INFO, WEBSITE};

use crate::{
    cli::helpers::render_row,
    menu_renderer::{DISCORD, GITHUB},
};

mod command;
mod helpers;

#[derive(Parser)]
#[cfg_attr(target_os = "windows", command(name = ".\\quantum_launcher.exe"))]
#[cfg_attr(not(target_os = "windows"), command(name = "./quantum_launcher"))]
#[command(version = LAUNCHER_VERSION_NAME)]
#[command(long_about = long_about())]
#[command(author = "Mrmayman")]
struct Cli {
    #[clap(subcommand)]
    command: Option<QSubCommand>,
    /// Some systems mistakenly pass this. It's unused though.
    #[arg(long, hide = true)]
    no_sandbox: Option<bool>,
    #[arg(long)]
    no_redact_info: bool,
    #[arg(short, long)]
    #[arg(help = "Manage servers, instead of instances")]
    #[arg(hide = true)]
    server: bool,
}

#[derive(Subcommand)]
enum QSubCommand {
    #[command(about = "Creates a new Minecraft instance")]
    Create {
        instance_name: String,
        #[arg(help = "Version of Minecraft to download")]
        version: String,
        #[arg(short, long)]
        #[arg(help = "Skips downloading game assets (sound/music) to speed up downloads")]
        skip_assets: bool,
    },
    #[command(about = "Launches an instance")]
    Launch {
        instance_name: String,
        #[arg(help = "Username to play with")]
        username: String,
        #[arg(short, long, short_alias = 'a')]
        #[arg(help = "Whether to use a logged in account of the given username (if any)")]
        use_account: bool,
    },
    #[command(aliases = ["list", "list-instances"], short_flag = 'l')]
    #[command(about = "Lists installed instances")]
    ListInstalled { properties: Option<Vec<String>> },
    #[command(about = "Deletes the specified instance")]
    Delete {
        instance_name: String,
        #[arg(short, long)]
        #[arg(help = "Forces deletion without confirmation. DANGEROUS")]
        force: bool,
    },
    #[clap(subcommand)]
    Loader(QLoader),
    #[command(about = "Lists downloadable versions", short_flag = 'a')]
    ListAvailableVersions,
}

#[derive(Subcommand)]
#[command(about = "Manages mod loaders")]
enum QLoader {
    Info {
        instance: String,
    },
    Install {
        instance: String,
        loader: String,
        #[arg(long)]
        version: Option<String>,
    },
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
    let cli = Cli::parse();
    *REDACT_SENSITIVE_INFO.lock().unwrap() = !cli.no_redact_info;
    if let Some(subcommand) = cli.command {
        if is_dir_err {
            std::process::exit(1);
        }
        let runtime = tokio::runtime::Runtime::new().unwrap();

        match subcommand {
            QSubCommand::Create {
                instance_name,
                version,
                skip_assets,
            } => {
                quit(command::create_instance(
                    instance_name,
                    version,
                    skip_assets,
                    &runtime,
                    cli.server,
                ));
            }
            QSubCommand::Launch {
                instance_name,
                username,
                use_account,
            } => {
                quit(command::launch_instance(
                    instance_name,
                    username,
                    use_account,
                    &runtime,
                    cli.server,
                ));
            }
            QSubCommand::ListAvailableVersions => {
                command::list_available_versions();
                std::process::exit(0);
            }
            QSubCommand::Delete {
                instance_name,
                force,
            } => quit(command::delete_instance(instance_name, force)),
            QSubCommand::ListInstalled { properties } => {
                quit(command::list_instances(properties.as_deref(), cli.server))
            }
            QSubCommand::Loader(cmd) => {
                quit(command::loader(cmd, &runtime, cli.server));
            }
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
