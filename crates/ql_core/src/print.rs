use std::{
    fmt::Display,
    fs::{File, OpenOptions},
    io::{BufWriter, Write},
    sync::{LazyLock, Mutex},
};

use chrono::{Datelike, Timelike};
use regex::Regex;

use crate::file_utils;

// TODO: Have an actually solid way to
// deal with cross platform color bullshit
//
// crossterm? owo-colors? Win32 API (shown below)?
//
// This might fix colors on windows, I have no clue
/*
#[cfg(windows)]
fn enable_ansi_support() {
    use std::io;
    use std::ptr;
    use winapi::um::consoleapi::GetConsoleMode;
    use winapi::um::consoleapi::SetConsoleMode;
    use winapi::um::processenv::GetStdHandle;
    use winapi::um::winbase::STD_OUTPUT_HANDLE;
    use winapi::um::wincon::ENABLE_VIRTUAL_TERMINAL_PROCESSING;

    unsafe {
        let handle = GetStdHandle(STD_OUTPUT_HANDLE);
        let mut mode = 0;
        if GetConsoleMode(handle, &mut mode) != 0 {
            SetConsoleMode(handle, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING);
        }
    }
}
*/

#[derive(Clone, Copy)]
pub enum LogType {
    Info,
    Error,
    Point,
}

impl Display for LogType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                LogType::Info => "[info]",
                LogType::Error => "[error]",
                LogType::Point => "-",
            }
        )
    }
}

pub struct LoggingState {
    thread: Option<std::thread::JoinHandle<()>>,
    writer: Option<BufWriter<File>>,
    sender: Option<std::sync::mpsc::Sender<String>>,
    pub text: Vec<(String, LogType)>,
}

impl LoggingState {
    #[must_use]
    pub fn create() -> Option<Mutex<LoggingState>> {
        Some(Mutex::new(LoggingState {
            thread: None,
            writer: None,
            sender: None,
            text: Vec::new(),
        }))
    }

    pub fn write_to_storage(&mut self, s: &str, t: LogType) {
        self.text.push((s.to_owned(), t));
    }

    pub fn write_str(&mut self, s: &str, t: LogType) {
        self.write_to_storage(s, t);

        if self.sender.is_none() {
            let (sender, receiver) = std::sync::mpsc::channel::<String>();

            if self.writer.is_none() {
                if let Some(file) = get_logs_file() {
                    self.writer = Some(BufWriter::new(file));
                }
            }

            if let Some(writer) = self.writer.take() {
                let thread = std::thread::spawn(move || {
                    let mut writer = writer;

                    while let Ok(msg) = receiver.recv() {
                        _ = writer.write_all(msg.as_bytes());
                        _ = writer.flush();
                    }
                });
                self.thread = Some(thread);
            }

            self.sender = Some(sender);
        }

        if let Some(sender) = &self.sender {
            _ = sender.send(s.to_owned());
        }
    }

    pub fn finish(&self) {
        if let Some(writer) = &self.writer {
            _ = writer.get_ref().sync_all();
        }
    }
}

fn get_logs_file() -> Option<File> {
    let logs_dir = file_utils::get_launcher_dir().ok()?.join("logs");
    std::fs::create_dir_all(&logs_dir).ok()?;
    let now = chrono::Local::now();
    let log_file_name = format!(
        "{}-{}-{}-{}-{}-{}.log",
        now.year(),
        now.month(),
        now.day(),
        now.hour(),
        now.minute(),
        now.second()
    );
    let log_file_path = logs_dir.join(log_file_name);
    let file = OpenOptions::new()
        .create(true) // Create file if it doesn't exist
        .append(true) // Append to the file instead of overwriting
        .open(&log_file_path)
        .ok()?;
    Some(file)
}

pub static LOGGER: LazyLock<Option<Mutex<LoggingState>>> = LazyLock::new(LoggingState::create);

pub fn print_to_file(msg: &str, t: LogType) {
    if let Some(logger) = LOGGER.as_ref() {
        if let Ok(mut lock) = logger.lock() {
            lock.write_str(msg, t);
        } else {
            eprintln!("ql_core::print::print_to_file(): Logger thread panicked!\n[msg]: {msg}");
        }
    }
}

pub fn logger_finish() {
    if let Some(logger) = LOGGER.as_ref() {
        if let Ok(lock) = logger.lock() {
            lock.finish();
        } else {
            eprintln!("ql_core::print::logger_finish(): Logger thread panicked!");
        }
    }
}

pub fn print_to_storage(msg: &str, t: LogType) {
    if let Some(logger) = LOGGER.as_ref() {
        if let Ok(mut lock) = logger.lock() {
            lock.write_to_storage(msg, t);
        } else {
            eprintln!("ql_core::print::print_to_storage(): Logger thread panicked!");
        }
    }
}

/// Print an informational message.
/// Saved to a log file.
#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {{
        let plain_text = $crate::print::strip_ansi_codes(&format!("{}", format_args!($($arg)*)));
        println!("{} {}", owo_colors::OwoColorize::yellow(&"[info]"), format_args!($($arg)*));
        $crate::print::print_to_file(&plain_text, $crate::print::LogType::Info);
    }};
}

/// Print an informational message.
/// Not saved to a log file.
#[macro_export]
macro_rules! info_no_log {
    ($($arg:tt)*) => {{
        let plain_text = $crate::print::strip_ansi_codes(&format!("{}", format_args!($($arg)*)));
        println!("{} {}", owo_colors::OwoColorize::yellow(&"[info]"), format_args!($($arg)*));
        $crate::print::print_to_storage(&plain_text, $crate::print::LogType::Info);
    }};
}

/// Print an error message.
/// Not saved to a log file.
#[macro_export]
macro_rules! err_no_log {
    ($($arg:tt)*) => {{
        let plain_text = $crate::print::strip_ansi_codes(&format!("{}", format_args!($($arg)*)));
        eprintln!("{} {}", owo_colors::OwoColorize::red(&"[error]"), format_args!($($arg)*));
        $crate::print::print_to_storage(&plain_text, $crate::print::LogType::Error);
    }};
}

/// Print an error message.
/// Saved to a log file.
#[macro_export]
macro_rules! err {
    ($($arg:tt)*) => {{
        let plain_text = $crate::print::strip_ansi_codes(&format!("{}", format_args!($($arg)*)));
        eprintln!("{} {}", owo_colors::OwoColorize::red(&"[error]"), format_args!($($arg)*));
        $crate::print::print_to_file(&plain_text, $crate::print::LogType::Error);
    }};
}

/// Print a point message, i.e. a small step in some process.
/// Saved to a log file.
#[macro_export]
macro_rules! pt {
    ($($arg:tt)*) => {{
        let plain_text = $crate::print::strip_ansi_codes(&format!("{}", format_args!($($arg)*)));
        println!("{} {}", owo_colors::OwoColorize::bold(&"-"), format_args!($($arg)*));
        $crate::print::print_to_file(&plain_text, $crate::print::LogType::Point);
    }};
}

/// Removes ANSI escape codes (colors, formatting, cursor moves) from a string.
pub fn strip_ansi_codes(input: &str) -> String {
    // Regex: ESC [ ... letters
    // ESC = \x1B or \u{1b}
    let re = Regex::new(r"\x1B\[[0-9;]*[A-Za-z]").unwrap();
    re.replace_all(input, "").to_string()
}
