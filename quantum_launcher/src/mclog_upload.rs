use ql_core::{
    CLIENT, InstanceConfigJson, Instance, IntoJsonError, IntoStringError, Loader,
    json::VersionDetails, request::check_for_success,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct MclogsResponse {
    pub success: bool,
    // pub id: Option<String>,
    pub url: Option<String>,
    // pub raw: Option<String>,
    pub error: Option<String>,
}

/// Uploads log content to <https://mclo.gs> and returns the URL if successful
pub async fn upload_log(content: String, instance: Instance) -> Result<String, String> {
    #[derive(serde::Serialize)]
    struct Metadata {
        key: &'static str,
        value: String,
        label: &'static str,
    }

    if content.trim().is_empty() {
        return Err("Cannot upload empty log".to_owned());
    }

    let (details, config) = tokio::try_join!(
        VersionDetails::load(&instance),
        InstanceConfigJson::read(&instance)
    )
    .strerr()?;

    let mut metadata = vec![Metadata {
        key: "version",
        value: details.id,
        label: "Minecraft version",
    }];

    if config.mod_type != Loader::Vanilla {
        metadata.push(Metadata {
            key: "loader",
            value: config.mod_type.to_string(),
            label: "Mod Loader",
        });
    }

    let response = CLIENT
        .post("https://api.mclo.gs/1/log")
        .json(&serde_json::json!({
            "content": content,
            "source": "mrmayman.github.io/quantumlauncher",
            "metadata": metadata,
        }))
        .send()
        .await
        .strerr()?;

    check_for_success(&response).strerr()?;
    let response_text = response.text().await.strerr()?;

    let mclog_response: MclogsResponse = serde_json::from_str(&response_text)
        .json(response_text)
        .strerr()?;

    if mclog_response.success {
        mclog_response
            .url
            .ok_or_else(|| "No URL in response".to_string())
    } else {
        Err(mclog_response
            .error
            .unwrap_or_else(|| "Unknown error".to_string()))
    }
}
