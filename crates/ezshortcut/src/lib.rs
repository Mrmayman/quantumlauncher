use std::path::{Path, PathBuf};

mod os;
pub use os::{get_menu_path, EXTENSION};

pub fn get_desktop_dir() -> Option<PathBuf> {
    dirs::desktop_dir().or_else(|| dirs::home_dir().map(|n| n.join("Desktop")))
}

#[derive(Debug, Clone)]
pub struct Shortcut {
    pub name: String,
    /// Linux/BSD/Unix only, leave blank for none
    pub description: String,
    pub exec: String,
    pub exec_args: Vec<String>,
    pub icon: Option<String>,
}

impl Shortcut {
    /// Create the shortcut and save it to `path`.
    ///
    /// Note: On Linux, if you just place the shortcut anywhere,
    /// it may not work. You may need to save it to system-wide locations
    /// (see [`Self::generate_to_applications`]).
    pub async fn generate(&self, path: &Path) -> std::io::Result<()> {
        os::create(self, path).await
    }

    /// Creates the shortcut and adds it to Start Menu/Applications/Application Menu.
    ///
    /// This also *tries* to refresh the application list, it may take a while to update though.
    pub async fn generate_to_applications(&self) -> std::io::Result<()> {
        os::create_in_applications(self).await
    }

    /// Gets the recommended filename for the shortcut based on OS behavior.
    pub fn get_filename(&self) -> String {
        let mut filtered_name = make_filename_safe(&self.name, !cfg!(target_os = "windows"));
        filtered_name.push_str(EXTENSION);
        filtered_name
    }

    fn get_formatted_args(&self) -> String {
        let mut s = String::new();
        for e in self.exec_args.iter().map(|n| format!("{n:?}")) {
            s.push_str(&e);
            s.push(' ');
        }
        s
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
