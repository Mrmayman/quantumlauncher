use std::{
    path::PathBuf,
    sync::{LazyLock, OnceLock},
};

use futures::StreamExt;
use http_cache_reqwest::{CACacheManager, Cache, CacheMode, HttpCache, HttpCacheOptions};
use reqwest::Client;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use tokio_util::io::StreamReader;

use crate::{
    DownloadFileError, IntoIoError, IntoJsonError, JsonDownloadError, LAUNCHER_CACHE_DIR,
    RETRY_LIMIT, RequestError, retry,
};

pub static CLIENT: OnceLock<ClientWithMiddleware> = OnceLock::new();
pub static CLIENT_UNCACHED: LazyLock<Client> = LazyLock::new(Client::new);

#[must_use]
pub fn build_middleware(path: PathBuf, cache: bool) -> ClientWithMiddleware {
    ClientBuilder::new(Client::new())
        .with(Cache(HttpCache {
            mode: if cache {
                CacheMode::Default
            } else {
                CacheMode::NoStore
            },
            manager: CACacheManager::new(path, false),
            options: HttpCacheOptions::default(),
        }))
        .build()
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

    async fn send(&self) -> Result<reqwest::Response, RequestError> {
        let client =
            CLIENT.get_or_init(|| build_middleware(LAUNCHER_CACHE_DIR.to_path_buf(), true));
        let mut get = client.get(self.url);

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
        check_for_success(&response)?;
        Ok(response)
    }

    pub async fn bytes(&self) -> Result<Vec<u8>, RequestError> {
        retry(|| async {
            let response = self.send().await?;
            Ok(response.bytes().await?.to_vec())
        })
        .await
    }

    pub async fn bytes_with_progress(
        &self,
        mut progress: impl FnMut(f32),
    ) -> Result<Vec<u8>, RequestError> {
        for attempt in 0..=RETRY_LIMIT {
            let result = async {
                progress(0.0);
                let response = self.send().await?;
                let total = response.content_length();

                let mut stream = response.bytes_stream();
                let mut bytes = Vec::new();
                let mut downloaded = 0u64;

                while let Some(chunk) = stream.next().await {
                    let chunk = chunk?;
                    downloaded += chunk.len() as u64;
                    bytes.extend_from_slice(&chunk);

                    if let Some(total) = total.filter(|&n| n > 0) {
                        let percentage =
                            (downloaded as f64 / total as f64 * 100.0).min(100.0) as f32;

                        progress(percentage);
                    }
                }

                progress(100.0);
                Ok(bytes)
            }
            .await;

            if result.is_ok() || attempt == RETRY_LIMIT {
                return result;
            }
        }

        unreachable!()
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
        if json_raw.is_empty() {
            return Err(JsonDownloadError::EmptyResponse(self.url.to_owned()));
        }
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

    /// Downloads a file directly to the specified path, reporting progress
    /// as a percentage from 0.0 to 100.0.
    pub async fn path_with_progress(
        &self,
        path: impl AsRef<std::path::Path>,
        mut progress: impl FnMut(f32),
    ) -> Result<(), DownloadFileError> {
        let path = path.as_ref();

        for attempt in 0..=RETRY_LIMIT {
            let result = async {
                progress(0.0);
                let response = self.send().await?;
                let total = response.content_length();

                if let Some(parent) = path.parent() {
                    if !parent.is_dir() {
                        tokio::fs::create_dir_all(parent).await.path(parent)?;
                    }
                }

                let mut file = tokio::fs::File::create(path).await.path(path)?;

                let mut stream = response.bytes_stream();
                let mut downloaded = 0u64;

                while let Some(chunk) = stream.next().await {
                    let chunk = chunk.map_err(RequestError::ReqwestError)?;
                    downloaded += chunk.len() as u64;

                    tokio::io::AsyncWriteExt::write_all(&mut file, &chunk)
                        .await
                        .map_err(|error| crate::IoError::FromUrl {
                            error,
                            path: path.to_owned(),
                            url: self.url.to_owned(),
                        })?;

                    if let Some(total) = total.filter(|&n| n > 0) {
                        let percentage =
                            (downloaded as f64 / total as f64 * 100.0).min(100.0) as f32;

                        progress(percentage);
                    }
                }

                progress(100.0);
                Ok(())
            }
            .await;

            if result.is_ok() || attempt == RETRY_LIMIT {
                return result;
            }
        }

        unreachable!()
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

pub fn check_for_success(r: &reqwest::Response) -> Result<(), RequestError> {
    if r.status().is_success() {
        Ok(())
    } else {
        Err(RequestError::DownloadError {
            code: r.status(),
            url: r.url().clone(),
        })
    }
}
