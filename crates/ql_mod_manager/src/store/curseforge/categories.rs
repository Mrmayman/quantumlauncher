use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
};

use ql_core::IntoJsonError;
use serde::Deserialize;

use crate::store::ModError;

use super::{get_mc_id, send_request};

#[derive(Deserialize, Clone, Debug)]
pub struct Categories {
    pub data: Vec<CfCategory>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CfCategory {
    pub id: i32,
    pub name: String,
    pub slug: String,
    pub parent_category_id: Option<i32>,
}

pub static CATEGORIES: LazyLock<Mutex<Option<Categories>>> = LazyLock::new(|| Mutex::new(None));

pub async fn get_categories() -> Result<Categories, ModError> {
    {
        let categories = CATEGORIES.lock().unwrap().clone();
        if let Some(categories) = categories {
            return Ok(categories);
        }
    }
    let mc_id = get_mc_id().await?;
    let params = HashMap::from([("gameId", mc_id.to_string())]);
    let categories = send_request("categories", &params).await?;
    let categories: Categories = serde_json::from_str(&categories).json(categories)?;

    *CATEGORIES.lock().unwrap() = Some(categories.clone());
    Ok(categories)
}
