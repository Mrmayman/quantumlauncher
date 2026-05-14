use std::sync::LazyLock;

use futures::StreamExt;
use http_cache_reqwest::{CACacheManager, Cache, CacheMode, HttpCache, HttpCacheOptions};
use reqwest_d::Client;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use tokio_util::io::StreamReader;

use crate::{
    DownloadFileError, IntoIoError, IntoJsonError, JsonDownloadError, LAUNCHER_CACHE_DIR,
    RequestError, pt, retry,
};

pub static DOWNLOAD_CLIENT: LazyLock<ClientWithMiddleware> = LazyLock::new(build_download_client);

fn build_download_client() -> ClientWithMiddleware {
    pt!(
        no_log,
        "Using {LAUNCHER_CACHE_DIR:?} as downloadables' cache directory."
    );

    let client = ClientBuilder::new(Client::new())
        .with(Cache(HttpCache {
            mode: CacheMode::Default,
            manager: CACacheManager::new(LAUNCHER_CACHE_DIR.to_path_buf(), false),
            options: HttpCacheOptions::default(),
        }))
        .build();

    client
}

#[must_use]
pub struct DownloadRequest<'a> {
    url: &'a str,
    user_agent: UserAgentKind,
}

impl DownloadRequest<'_> {
    pub fn user_agent_spoof(mut self) -> Self {
        self.user_agent = UserAgentKind::Spoofed;
        self
    }

    pub fn user_agent_ql(mut self) -> Self {
        self.user_agent = UserAgentKind::Ql;
        self
    }

    async fn send(&self) -> Result<reqwest_d::Response, RequestError> {
        let mut get = DOWNLOAD_CLIENT.get(self.url);
        match self.user_agent {
            UserAgentKind::None => {}
            UserAgentKind::Ql => {
                get = get.header(
                    "User-Agent",
                    "Mrmayman/quantumlauncher (https://mrmayman.github.io/quantumlauncher)",
                );
            }
            UserAgentKind::Spoofed => {
                get = get.header(
                    "User-Agent",
                    "Mozilla/5.0 (X11; Linux x86_64; rv:148.0) Gecko/20100101 Firefox/148.0",
                );
            }
        }
        let response = get.send().await?;
        check_for_success(ResponseType::Download(&response))?;
        Ok(response)
    }

    pub async fn bytes(&self) -> Result<Vec<u8>, RequestError> {
        retry(|| async {
            let response = self.send().await?;
            Ok(response.bytes().await?.to_vec())
        })
        .await
    }

    pub async fn string(&self) -> Result<String, RequestError> {
        retry(|| async {
            let response = self.send().await?;
            Ok(response.text().await?)
        })
        .await
    }

    pub async fn json<T: serde::de::DeserializeOwned>(&self) -> Result<T, JsonDownloadError> {
        let json_raw = self.string().await?;
        Ok(serde_json::from_str(&json_raw).json(json_raw)?)
    }

    /// Downloads file directly to specified path, not storing it in memory.
    ///
    /// This uses `tokio` streams internally allowing for highly
    /// efficient downloading.
    ///
    /// # Errors
    /// - Error sending request
    /// - Request is rejected (HTTP status code)
    /// - Redirect loop detected
    /// - Redirect limit exhausted.
    pub async fn path(&self, path: impl AsRef<std::path::Path>) -> Result<(), DownloadFileError> {
        retry(|| async {
            let response = self.send().await?;

            let stream = response
                .bytes_stream()
                .map(|n| n.map_err(std::io::Error::other));
            let mut stream = StreamReader::new(stream);

            let path = path.as_ref();
            if let Some(parent) = path.parent() {
                if !parent.is_dir() {
                    tokio::fs::create_dir_all(&parent).await.path(parent)?;
                }
            }

            let mut file = tokio::fs::File::create(&path).await.path(path)?;
            tokio::io::copy(&mut stream, &mut file)
                .await
                .map_err(|error| crate::IoError::FromUrl {
                    error,
                    path: path.to_owned(),
                    url: self.url.to_owned(),
                })?;
            Ok(())
        })
        .await
    }
}

enum UserAgentKind {
    None,
    Ql,
    Spoofed,
}

pub fn download(url: &str) -> DownloadRequest<'_> {
    DownloadRequest {
        url,
        user_agent: UserAgentKind::None,
    }
}

pub enum ResponseType<'a> {
    Regular(&'a reqwest::Response),
    Download(&'a reqwest_d::Response),
}

/// It is advised to use `ResponseType::Regular` variant because `ResponseType::Download`
/// cannot and should not be used outside of the `ql_core` module. It will automatically be
/// used during `download()` calls.
pub fn check_for_success(response: ResponseType) -> Result<(), RequestError> {
    match response {
        ResponseType::Regular(response) => {
            if response.status().is_success() {
                Ok(())
            } else {
                Err(RequestError::DownloadError {
                    code: response.status(),
                    url: response.url().clone(),
                })
            }
        }
        ResponseType::Download(response) => {
            if response.status().is_success() {
                Ok(())
            } else {
                Err(RequestError::DownloadError {
                    code: response.status(),
                    url: response.url().clone(),
                })
            }
        }
    }
}
