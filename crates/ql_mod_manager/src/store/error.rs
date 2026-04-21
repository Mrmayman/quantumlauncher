use std::{fmt::Display, num::ParseIntError};

use ql_core::{IoError, JsonError, RequestError, impl_3_errs_jri};
use thiserror::Error;

use crate::store::QueryType;

use super::modpack::PackError;

const MOD_ERR_PREFIX: &str = "while managing mods:\n";

#[derive(Debug, Error)]
pub enum ModError {
    #[error("{MOD_ERR_PREFIX}{0}")]
    RequestError(#[from] RequestError),
    #[error("{MOD_ERR_PREFIX}{0}")]
    Json(#[from] JsonError),
    #[error("{MOD_ERR_PREFIX}{0}")]
    Io(#[from] IoError),

    #[error("{MOD_ERR_PREFIX}no compatible version found for mod: {0}")]
    NoCompatibleVersionFound(String),
    #[error("{MOD_ERR_PREFIX}no valid files found for mod")]
    NoFilesFound,
    #[error(
        "{MOD_ERR_PREFIX}unknown project_type while downloading from store: {0}\n\nThis is a bug, please report in discord!"
    )]
    UnknownProjectType(String),
    #[error(
        "{MOD_ERR_PREFIX}no \"minecraft\" game entry found in curseforge API\n\nThis is a bug, please report in discord!"
    )]
    NoMinecraftInCurseForge,
    #[error(
        "curseforge is blocking you from downloading the mod {0}\nGo to the official website at:\nhttps://www.curseforge.com/minecraft/mc-mods/{1}\nand download from there"
    )]
    CurseforgeModNotAllowedForDownload(String, String),

    #[error(
        "{MOD_ERR_PREFIX}no category {0} found in curseforge API\n\nThis is a bug, please report in discord!"
    )]
    CfCategoryNotFound(QueryType),

    #[error("{MOD_ERR_PREFIX}couldn't add entry {1} to zip: {0}")]
    ZipIoError(std::io::Error, String),
    #[error("{MOD_ERR_PREFIX}zip error:\n{0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("while checking for mod update:\ncould not parse date:\n{0}")]
    Chrono(#[from] chrono::ParseError),
    #[error("{MOD_ERR_PREFIX}couldn't parse int (curseforge mod id):\n{0}")]
    ParseInt(#[from] ParseIntError),

    #[error("{MOD_ERR_PREFIX}{0}")]
    Pack(#[from] Box<PackError>),
    #[error("{MOD_ERR_PREFIX}not a valid modpack or QMP preset!")]
    NotValidPack,
    #[error("{MOD_ERR_PREFIX}API Error: {error_id}\n{description}")]
    ApiError {
        error_id: String,
        description: String,
    },
}

impl_3_errs_jri!(ModError, Json, RequestError, Io);

impl From<reqwest::Error> for ModError {
    fn from(value: reqwest::Error) -> Self {
        Self::RequestError(RequestError::ReqwestError(value))
    }
}

#[derive(Debug)]
pub struct GameExpectation {
    pub expected: String,
    pub got: String,
}

impl Display for GameExpectation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.got == "Vanilla" {
            write!(
                f,
                "You don't have {exp} installed!\nPlease install {exp}",
                exp = self.expected
            )
        } else {
            write!(
                f,
                "Expected {expected}, got {got}\nPlease install {expected}",
                expected = self.expected,
                got = self.got
            )
        }
    }
}
