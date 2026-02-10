use std::path::Path;

mod async_impl;

pub struct Shortcut {
    pub name: String,
    pub description: String,
    pub exec: String,
    pub icon: Option<String>,
}

impl Shortcut {
    pub async fn generate(&self, path: &Path) -> std::io::Result<()> {
        async_impl::create(self, path).await
    }
}

fn make_filename_safe(input: &str, remove_spaces: bool) -> String {
    let mut out = String::with_capacity(input.len());

    for c in input.chars() {
        match c {
            '-' | '_' | '.' => out.push(c),
            ' ' | '/' | '\\' | '|' | ':' => out.push('_'),
            '<' | '>' | '"' | '\'' | '?' | '*' => continue,
            c if c.is_control() => continue,
            c if c.is_whitespace() => out.push(if remove_spaces { '_' } else { ' ' }),

            _ => out.push(c),
        }
    }

    // Collapse multiple underscores
    let mut collapsed = String::with_capacity(out.len());
    let mut last_was_underscore = false;

    for c in out.chars() {
        if c == '_' {
            if !last_was_underscore {
                collapsed.push(c);
                last_was_underscore = true;
            }
        } else {
            collapsed.push(c);
            last_was_underscore = false;
        }
    }

    // Trim leading/trailing spaces, dots, and underscores (Windows edge cases)
    let trimmed = collapsed
        .trim_matches(|c: char| c == ' ' || c == '.' || c == '_')
        .to_string();

    // Avoid reserved Windows filenames
    let upper = trimmed.to_ascii_uppercase();
    let reserved = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];

    if reserved.contains(&upper.as_str()) || trimmed.is_empty() {
        "_".to_string()
    } else {
        trimmed
    }
}
