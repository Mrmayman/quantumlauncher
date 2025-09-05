use std::path::{MAIN_SEPARATOR, MAIN_SEPARATOR_STR};

use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
pub struct FabricJSON {
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
    pub url: String,
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
    pub fn get_url(&self) -> String {
        let parts: Vec<&str> = self.name.split(':').collect();
        format!(
            "{}{}/{p1}/{p2}/{p1}-{p2}.jar",
            self.url,
            parts[0].replace('.', "/"),
            p1 = parts[1],
            p2 = parts[2],
        )
    }
}
