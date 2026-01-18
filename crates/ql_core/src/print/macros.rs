#[macro_export]
macro_rules! eeprintln {
    ($($arg:tt)*) => {{
        if *$crate::print::IS_GIT_BASH {
            println!("{}", format_args!($($arg)*));
        } else {
            eprintln!("{}", format_args!($($arg)*));
        }
    }};
}

/// Print an informational message
#[macro_export]
macro_rules! info {
    (no_log, $($arg:tt)*) => {{
        let msg = format!("{}", format_args!($($arg)*));
        let redacted = $crate::print::auto_redact(&msg);
        if $crate::print::is_print() {
            println!("{} {}", owo_colors::OwoColorize::yellow(&"[info]"), redacted);
        }
        $crate::print::print_to_memory(&redacted, $crate::print::LogType::Info);
    }};

    ($($arg:tt)*) => {{
        let msg = format!("{}", format_args!($($arg)*));
        let redacted = $crate::print::auto_redact(&msg);
        if $crate::print::is_print() {
            println!("{} {}", owo_colors::OwoColorize::yellow(&"[info]"), redacted);
        }
        $crate::print::print_to_file(&redacted, $crate::print::LogType::Info);
    }};
}

/// Print an error message
#[macro_export]
macro_rules! err {
    (no_log, $($arg:tt)*) => {{
        let msg = format!("{}", format_args!($($arg)*));
        let redacted = $crate::print::auto_redact(&msg);
        if $crate::print::is_print() {
            $crate::eeprintln!("{} {}", owo_colors::OwoColorize::red(&"[error]"), redacted);
        }
        $crate::print::print_to_memory(&redacted, $crate::print::LogType::Error);
    }};

    ($($arg:tt)*) => {{
        let msg = format!("{}", format_args!($($arg)*));
        let redacted = $crate::print::auto_redact(&msg);
        if $crate::print::is_print() {
            $crate::eeprintln!("{} {}", owo_colors::OwoColorize::red(&"[error]"), redacted);
        }
        $crate::print::print_to_file(&redacted, $crate::print::LogType::Error);
    }};
}

/// Print a point message, i.e. a small step in some process
#[macro_export]
macro_rules! pt {
    (no_log, $($arg:tt)*) => {{
        let msg = format!("{}", format_args!($($arg)*));
        let redacted = $crate::print::auto_redact(&msg);
        if $crate::print::is_print() {
            println!("{} {}", owo_colors::OwoColorize::bold(&"-"), redacted);
        }
        $crate::print::print_to_memory(&redacted, $crate::print::LogType::Point);
    }};

    ($($arg:tt)*) => {{
        let msg = format!("{}", format_args!($($arg)*));
        let redacted = $crate::print::auto_redact(&msg);
        if $crate::print::is_print() {
            println!("{} {}", owo_colors::OwoColorize::bold(&"-"), redacted);
        }
        $crate::print::print_to_file(&redacted, $crate::print::LogType::Point);
    }};
}
