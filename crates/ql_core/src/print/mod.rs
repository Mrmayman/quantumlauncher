use std::{
    fmt::Display,
    fs::{File, OpenOptions},
    io::{BufWriter, Write},
    sync::{LazyLock, RwLock},
};

use chrono::{Datelike, Timelike};
use regex::Regex;

use crate::{eeprintln, file_utils, REDACT_SENSITIVE_INFO};

pub mod macros;

/// Censor username for privacy
pub static REDACTION_USERNAME: LazyLock<(Vec<String>, String)> = LazyLock::new(|| {
    if let Some(home_dir) = dirs::home_dir() {
        if let (Some(home_str), Some(username)) = (
            home_dir.to_str(),
            home_dir.file_name().and_then(|n| n.to_str()),
        ) {
            return (
                vec![
                    home_str.to_owned(),
                    home_str.replace("\\", "/"),
                    home_str.replace("\\", "\\\\"),
                ],
                username.to_owned(),
            );
        }
    }
    (Vec::new(), String::new())
});

/// Automatically redact sensitive information from log messages.
/// This is called by all logging macros to ensure no username/path exposure.
pub fn auto_redact(message: &str) -> String {
    let mut redacted = message.to_string();
    if *REDACT_SENSITIVE_INFO.lock().unwrap() {
        let (home_dir, username) = &*REDACTION_USERNAME;
        if home_dir.iter().any(|n| message.contains(n)) {
            redacted = redacted.replace(username, "[REDACTED]");
        }
    }
    redacted
}

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

pub struct LogConfig {
    pub terminal: bool,
    pub file: bool,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            terminal: true,
            file: true,
        }
    }
}

#[derive(Default)]
pub struct LoggingState {
    thread: Option<std::thread::JoinHandle<()>>,
    writer: Option<BufWriter<File>>,
    sender: Option<std::sync::mpsc::Sender<String>>,
    config: LogConfig,
    pub text: Vec<(String, LogType)>,
}

impl LoggingState {
    #[must_use]
    pub fn create() -> Option<RwLock<LoggingState>> {
        Some(RwLock::new(Self::default()))
    }

    pub fn write_to_memory(&mut self, s: &str, t: LogType) {
        self.text.push((s.to_owned(), t));
    }

    pub fn write_to_logfile(&mut self, s: &str, t: LogType) {
        self.write_to_memory(s, t);

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
                        _ = writer.write_all(t.to_string().as_bytes());
                        _ = writer.write(b" ");
                        _ = writer.write_all(msg.as_bytes());
                        _ = writer.write(b"\n");
                        _ = writer.flush();
                    }
                });
                self.thread = Some(thread);
            }

            self.sender = Some(sender);
        }

        if let Some(sender) = &self.sender {
            if self.config.file {
                _ = sender.send(s.to_owned());
            }
        }
    }

    pub fn finish(&self) {
        if let Some(writer) = &self.writer {
            _ = writer.get_ref().sync_all();
        }
    }
}

pub fn set_config(c: LogConfig) {
    if let Some(l) = &*LOGGER {
        l.write().unwrap().config = c;
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

pub static LOGGER: LazyLock<Option<RwLock<LoggingState>>> = LazyLock::new(LoggingState::create);

pub fn get() -> Vec<(String, LogType)> {
    LOGGER
        .as_ref()
        .and_then(|l| l.read().ok())
        .map_or(Vec::new(), |n| n.text.clone())
}

pub fn print_to_file(msg: &str, t: LogType) {
    if let Some(logger) = LOGGER.as_ref() {
        if let Ok(mut lock) = logger.write() {
            lock.write_to_logfile(&strip_ansi_codes(msg), t);
        } else {
            eeprintln!("ql_core::print::print_to_file(): Logger thread panicked!\n[msg]: {msg}");
        }
    }
}

pub fn logger_finish() {
    if let Some(logger) = LOGGER.as_ref() {
        if let Ok(lock) = logger.write() {
            lock.finish();
        } else {
            eeprintln!("ql_core::print::logger_finish(): Logger thread panicked!");
        }
    }
}

pub fn print_to_memory(msg: &str, t: LogType) {
    if let Some(logger) = LOGGER.as_ref() {
        if let Ok(mut lock) = logger.write() {
            lock.write_to_memory(&strip_ansi_codes(msg), t);
        } else {
            eeprintln!("ql_core::print::print_to_memory(): Logger thread panicked!");
        }
    }
}

#[must_use]
pub fn is_print() -> bool {
    if let Some(l) = &*LOGGER {
        l.read().unwrap().config.terminal
    } else {
        true
    }
}

/// Regex: ESC [ ... letters
/// ESC = `\x1B` or `\u{1b}`
static ANSI_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\x1B\[[0-9;]*[A-Za-z]").unwrap());

/// Removes ANSI escape codes (colors, formatting, cursor moves) from a string.
pub fn strip_ansi_codes(input: &str) -> String {
    ANSI_REGEX.replace_all(input, "").to_string()
}

/// Used to fix a super annoying bug
pub static IS_GIT_BASH: LazyLock<bool> = LazyLock::new(|| {
    if cfg!(target_os = "windows") {
        std::env::var_os("MSYSTEM").is_some()
            || std::env::var_os("MSYS").is_some()
            || std::env::var_os("MINGW_PREFIX").is_some()
    } else {
        false
    }
});
