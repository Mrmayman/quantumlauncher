use std::{
    collections::HashMap,
    fmt::{Display, Write},
    process::ExitStatus,
    sync::{mpsc::Sender, Arc, Mutex},
};

use owo_colors::OwoColorize;
use serde::Deserialize;
use thiserror::Error;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{Child, ChildStderr, ChildStdout},
};

use crate::{
    err, json::VersionDetails, InstanceSelection, IoError, JsonError, JsonFileError,
    REDACT_SENSITIVE_INFO,
};

/// Reads log output from the given instance
/// and sends it to the given sender.
///
/// This async function runs till the instance process exits,
/// then it returns the exit status.
///
/// This automatically deals with XML logs.
///
/// # Arguments
/// - `stdout`: The stdout of the instance process.
/// - `stderr`: The stderr of the instance process.
/// - `child`: The instance process.
/// - `sender`: The sender to send [`LogLine`]s to.
/// - `instance_name`: The name of the instance.
/// - `censors`: Any strings to censor (like session id, password, etc.)
///   Leave blank if not needed.
///
/// # Errors
/// If:
/// - `details.json` couldn't be read or parsed into JSON
///   (for checking if XML logs are used)
/// - the `Receiver<LogLine>` was dropped,
///   disconnecting the channel
/// - Tokio *somehow* fails to read the `stdout` or `stderr`
#[allow(clippy::missing_panics_doc)]
pub(crate) async fn read_logs(
    stdout: ChildStdout,
    stderr: ChildStderr,
    child: Arc<Mutex<Child>>,
    sender: Option<Sender<LogLine>>,
    instance: InstanceSelection,
    censors: Vec<String>,
) -> Result<(ExitStatus, InstanceSelection), ReadError> {
    // TODO: Use the "newfangled" approach of the Modrinth launcher:
    // https://github.com/modrinth/code/blob/main/packages/app-lib/src/state/process.rs#L208
    //
    // It uses tokio and quick_xml's async features.
    // It also looks a lot less "magic" than my approach.
    // Also, the Modrinth app is GNU GPLv3 so I guess it's
    // safe for me to take some code.

    let uses_xml = !instance.is_server() && is_xml(instance.get_name()).await?;

    let mut stdout_reader = BufReader::new(stdout).lines();
    let mut stderr_reader = BufReader::new(stderr).lines();

    let mut xml_cache = String::new();

    let mut has_errored = false;

    loop {
        {
            let mut child = child.lock().unwrap();
            if let Ok(Some(status)) = child.try_wait() {
                // Game has exited.
                return Ok((status, instance));
            }
        };

        tokio::select! {
            line = stdout_reader.next_line() => {
                if let Some(mut line) = line? {
                    line = censor(&line, &censors);
                    if uses_xml {
                        xml_parse(sender.as_ref(), &mut xml_cache, &line, &mut has_errored);
                    } else {
                        if !line.ends_with('\n') {
                            line.push('\n');
                        }
                        send(sender.as_ref(), LogLine::Message(line));
                    }
                } // else EOF
            },
            line = stderr_reader.next_line() => {
                if let Some(mut line) = line? {
                    line = censor(&line, &censors);
                    if !line.ends_with('\n') {
                        line.push('\n');
                    }
                    send(sender.as_ref(), LogLine::Error(line));
                }
            }
        }
    }
}

fn censor(input: &str, censors: &[String]) -> String {
    if *REDACT_SENSITIVE_INFO.lock().unwrap() {
        censors.iter().fold(input.to_string(), |acc, censor| {
            acc.replace(censor, "[REDACTED]")
        })
    } else {
        input.to_string()
    }
}

fn send(sender: Option<&Sender<LogLine>>, msg: LogLine) {
    if let LogLine::Info(LogEvent {
        message: Some(message),
        ..
    }) = &msg
    {
        if message.contains("Session ID is ") {
            return;
        }
    }
    if let Some(sender) = sender {
        _ = sender.send(msg);
    } else {
        println!("{}", msg.print_colored());
    }
}

fn xml_parse(
    sender: Option<&Sender<LogLine>>,
    xml_cache: &mut String,
    line: &str,
    has_errored: &mut bool,
) {
    if !line.starts_with("  </log4j:Event>") {
        xml_cache.push_str(line);
        return;
    }

    xml_cache.push_str(line);
    let xml = xml_cache.replace("<log4j:", "<").replace("</log4j:", "</");
    let start = xml.find("<Event");

    let text = match start {
        Some(start) if start > 0 => {
            let other_text = xml[..start].trim();
            if !other_text.is_empty() {
                send(sender, LogLine::Message(other_text.to_owned()));
            }
            &xml[start..]
        }
        _ => &xml,
    };

    if let Ok(mut log_event) = quick_xml::de::from_str::<LogEvent>(text) {
        log_event.fix_tabs();
        send(sender, LogLine::Info(log_event));
        xml_cache.clear();
    } else {
        let no_unicode = any_ascii::any_ascii(text);
        match quick_xml::de::from_str::<LogEvent>(&no_unicode) {
            Ok(mut log_event) => {
                log_event.fix_tabs();
                send(sender, LogLine::Info(log_event));
                xml_cache.clear();
            }
            Err(err) => {
                // Prevents HORRIBLE log spam
                // I once had a user complain about a 35 GB logs folder
                // because this thing printed the same error again and again
                if !*has_errored {
                    err!("Could not parse XML: {err}\n{text}\n");
                    *has_errored = true;
                }
            }
        }
    }
}

async fn is_xml(instance_name: &str) -> Result<bool, ReadError> {
    let json = VersionDetails::load(&InstanceSelection::Instance(instance_name.to_owned())).await?;

    Ok(json.logging.is_some())
}

/// Represents a line of log output.
///
/// # Variants
/// - `Info(LogEvent)`: A log event. Contains advanced
///   information about the log line like the timestamp,
///   class name, level and thread.
/// - `Message(String)`: A normal log message. Primarily
///   used for non-XML logs (old Minecraft versions).
/// - `Error(String)`: An error log message.
pub enum LogLine {
    Info(LogEvent),
    Message(String),
    Error(String),
}

impl LogLine {
    #[must_use]
    pub fn print_colored(&self) -> String {
        match self {
            LogLine::Info(event) => event.print_color(),
            LogLine::Message(message) => message.clone(),
            LogLine::Error(error) => error.bright_red().to_string(),
        }
    }
}

impl Display for LogLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLine::Info(event) => write!(f, "{event}"),
            LogLine::Error(error) => write!(f, "{error}"),
            LogLine::Message(message) => write!(f, "{message}"),
        }
    }
}

const READ_ERR_PREFIX: &str = "while reading the game log:\n";

#[derive(Debug, Error)]
pub enum ReadError {
    #[error("{READ_ERR_PREFIX}{0}")]
    Io(#[from] std::io::Error),
    #[error("{READ_ERR_PREFIX}{0}")]
    IoError(#[from] IoError),
    #[error("{READ_ERR_PREFIX}{0}")]
    Json(#[from] JsonError),
}

impl From<JsonFileError> for ReadError {
    fn from(value: JsonFileError) -> Self {
        match value {
            JsonFileError::SerdeError(err) => err.into(),
            JsonFileError::Io(err) => err.into(),
        }
    }
}

/// Represents a log event.
/// Contains advanced information about the log line
/// like the timestamp, class name, level and thread.
/// This is used for XML logs.
#[derive(Debug, Deserialize)]
pub struct LogEvent {
    /// The Java Class that logged the message.
    /// It's usually obfuscated so not useful most of the time,
    /// but might be useful for debugging mod-related crashes.
    #[serde(rename = "@logger")]
    pub logger: String,
    /// Logging timestamp in milliseconds,
    /// since the UNIX epoch.
    ///
    /// Use [`LogEvent::get_time`] to convert
    /// to `HH:MM:SS` time.
    #[serde(rename = "@timestamp")]
    pub timestamp: String,
    #[serde(rename = "@level")]
    pub level: String,
    #[serde(rename = "@thread")]
    pub thread: String,
    #[serde(rename = "Message")]
    pub message: Option<String>,
    #[serde(rename = "Throwable")]
    pub throwable: Option<String>,
}

impl LogEvent {
    /// Returns the time of the log event, formatted as `HH:MM:SS`.
    #[must_use]
    pub fn get_time(&self) -> Option<String> {
        let time: i64 = self.timestamp.parse().ok()?;
        let seconds = time / 1000;
        let milliseconds = time % 1000;
        let nanoseconds = milliseconds * 1_000_000;
        let datetime = chrono::DateTime::from_timestamp(seconds, nanoseconds as u32)?;
        let datetime = datetime.with_timezone(&chrono::Local);
        Some(datetime.format("%H:%M:%S").to_string())
    }

    #[must_use]
    pub fn print_color(&self) -> String {
        let date = self.get_time().unwrap_or_else(|| self.timestamp.clone());

        let bright_black = self.level.bright_black();
        let level = bright_black.underline();
        // let thread = self.thread.bright_black().underline();
        // let class = self.logger.bright_black().underline();

        let mut out = format!(
            // "{b1}{level}{b2} {b1}{date}{c}{thread}{c}{class}{b2} {msg}",
            "{b1}{level}{b2} {b1}{date}{b2} {msg}",
            b1 = "[".bright_black(),
            b2 = "]".bright_black(),
            // c = ":".bright_black(),
            msg = if let Some(n) = &self.message {
                if cfg!(target_os = "windows") {
                    n.clone()
                } else {
                    parse_color(n)
                }
            } else {
                String::new()
            }
        );
        if let Some(throwable) = self.throwable.as_deref() {
            let throwable = throwable.replace('\t', "\n\t");
            _ = writeln!(out, "\nCaused by {throwable}");
        }
        out
    }

    pub fn fix_tabs(&mut self) {
        if let Some(message) = &mut self.message {
            *message = message.replace('\t', "\n\t");
        }
    }
}

fn parse_color(msg: &str) -> String {
    let color_map: HashMap<char, &str> = [
        // Colors
        ('0', "\x1b[30m"), // Black
        ('1', "\x1b[34m"), // Dark Blue
        ('2', "\x1b[32m"), // Dark Green
        ('3', "\x1b[36m"), // Dark Aqua
        ('4', "\x1b[31m"), // Dark Red
        ('5', "\x1b[35m"), // Dark Purple
        ('6', "\x1b[33m"), // Gold
        ('7', "\x1b[37m"), // Gray
        ('8', "\x1b[90m"), // Dark Gray
        ('9', "\x1b[94m"), // Blue
        ('a', "\x1b[92m"), // Green
        ('b', "\x1b[96m"), // Aqua
        ('c', "\x1b[91m"), // Red
        ('d', "\x1b[95m"), // Light Purple
        ('e', "\x1b[93m"), // Yellow
        ('f', "\x1b[97m"), // White
        // Formatting
        ('l', "\x1b[1m"), // Bold
        ('m', "\x1b[9m"), // Strikethrough
        ('n', "\x1b[4m"), // Underline
        ('o', "\x1b[3m"), // Italic
        ('r', "\x1b[0m"), // Reset
    ]
    .iter()
    .copied()
    .collect();

    let mut out = String::new();

    let mut iter = msg.chars();
    while let Some(c) = iter.next() {
        if c == '§' {
            let Some(format) = iter.next() else { break };
            if let Some(color) = color_map.get(&format) {
                out.push_str(color);
            } else {
                out.push('§');
                out.push(format);
            }
        } else {
            out.push(c);
        }
    }

    out.push_str("\x1b[0m");
    out
}

impl Display for LogEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let date = self.get_time().unwrap_or_else(|| self.timestamp.clone());
        writeln!(
            f,
            // "[{level}] [{date}:{thread}:{class}] {msg}",
            "[{level}] [{date}] {msg}",
            level = self.level,
            // thread = self.thread,
            // class = self.logger,
            msg = if let Some(n) = &self.message { &n } else { "" }
        )?;
        if let Some(throwable) = self.throwable.as_deref() {
            let throwable = throwable.replace('\t', "\n\t");
            writeln!(f, "Caused by {throwable}")?;
        }
        Ok(())
    }
}

// "Better" implementation of this whole damn thing
// using `std::io::pipe`, which was added in Rust 1.87.0
// It is cleaner and more elegant, but... my MSRV :(
/*
pub async fn read_logs(
    stream: PipeReader,
    child: Arc<Mutex<(Child, Option<PipeReader>)>>,
    sender: Sender<LogLine>,
    instance_name: String,
) -> Result<(ExitStatus, String), ReadError> {
    let uses_xml = is_xml(&instance_name).await?;
    let mut xml_cache = String::new();

    let mut stream = BufReader::new(stream);

    loop {
        let mut line = String::new();
        let bytes = stream.read_line(&mut line).map_err(ReadError::Io)?;

        if bytes == 0 {
            let status = {
                let mut child = child.lock().unwrap();
                child.0.try_wait()
            };
            if let Ok(Some(status)) = status {
                // Game has exited.
                if !xml_cache.is_empty() {
                    sender.send(LogLine::Message(xml_cache))?;
                }
                return Ok((status, instance_name));
            }
        } else {
            if uses_xml {
                xml_parse(&sender, &mut xml_cache, &line)?;
            } else {
                sender.send(LogLine::Message(line))?;
            }
        }
    }
}
*/
