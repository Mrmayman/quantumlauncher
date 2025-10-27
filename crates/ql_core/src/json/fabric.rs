use std::path::{MAIN_SEPARATOR, MAIN_SEPARATOR_STR};

use serde::Deserialize;

use crate::json::version::LibraryRule;

#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
pub struct FabricJSON {
    pub mainClassServer: Option<String>,
    pub mainClass: String,
    pub arguments: Option<Arguments>,
    pub libraries: Vec<Library>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Arguments {
    pub jvm: Option<Vec<String>>,
    pub game: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Library {
    pub name: String,
    pub url: Option<String>,
    pub rules: Option<Vec<LibraryRule>>,
}

impl Library {
    #[must_use]
    pub fn get_path(&self) -> String {
        let parts: Vec<&str> = self.name.split(':').collect();
        format!(
            "{}{MAIN_SEPARATOR}{}{MAIN_SEPARATOR}{}{MAIN_SEPARATOR}{}-{}{MAIN_SEPARATOR}{}.jar",
            parts[0].replace('.', MAIN_SEPARATOR_STR),
            parts[1],
            parts[2],
            parts[1],
            parts[2],
            parts[0].replace(' ', "_")
        )
    }

    #[must_use]
    pub fn get_url(&self) -> Option<String> {
        let parts: Vec<&str> = self.name.split(':').collect();
        self.url.as_deref().map(|url| {
            format!(
                "{url}{}/{p1}/{p2}/{p1}-{p2}.jar",
                parts[0].replace('.', "/"),
                p1 = parts[1],
                p2 = parts[2],
            )
        })
    }

    pub fn is_allowed(&self) -> bool {
        crate::json::version::Library {
            downloads: None,
            extract: None,
            name: Some(self.name.clone()),
            rules: self.rules.clone(),
            natives: None,
            url: self.url.clone(),
        }
        .is_allowed()
    }

    pub fn is_lwjgl2(&self) -> bool {
        self.name.contains("org.lwjgl.lwjgl:") && self.name.contains(":2.")
    }
}
