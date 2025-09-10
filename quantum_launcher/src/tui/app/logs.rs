// Logs utilities for TUI: helpers to append and maintain buffers

/// Push a line into the provided buffer with a maximum length constraint.
/// Returns the new length.
pub fn push_capped(buf: &mut Vec<String>, line: String, cap: usize) -> usize {
    buf.push(line);
    if buf.len() > cap {
        buf.remove(0);
    }
    buf.len()
}

/// Mirror a log line into the core logger storage as Info, ensuring trailing newline.
pub fn mirror_to_core(mut line: String) {
    if !line.ends_with('\n') {
        line.push('\n');
    }
    ql_core::print::print_to_storage(&line, ql_core::print::LogType::Info);
}
