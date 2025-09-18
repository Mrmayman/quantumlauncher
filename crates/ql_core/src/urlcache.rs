use sha2::{Digest, Sha256};
use tokio::{fs, io::AsyncWriteExt};

use crate::{file_utils, DownloadFileError, IntoIoError, LAUNCHER_DIR};

pub async fn url_cache_get(url: &str) -> Result<Vec<u8>, DownloadFileError> {
    let hash = hash(&url);

    let cache_dir = LAUNCHER_DIR.join("downloads/cache");
    fs::create_dir_all(&cache_dir).await.path(&cache_dir)?;

    let cache_file = cache_dir.join(&hash);
    let tmp_file = cache_dir.join(format!(".temp-{hash}"));

    if fs::try_exists(&cache_file).await.path(&cache_file)? {
        return Ok(fs::read(&cache_file).await.path(&cache_file)?);
    }

    let bytes = match file_utils::download_file_to_bytes(&url, true).await {
        Ok(n) => n,
        Err(_) => {
            // WTF: Some pesky cloud provider might be
            // blocking the launcher because they think it's a bot.

            // I understand people do this to protect
            // their servers but what this is doing is clearly
            // not malicious. We're just downloading some images :)

            file_utils::download_file_to_bytes_with_agent(
                &url,
                "Mozilla/5.0 (X11; Linux x86_64; rv:135.0) Gecko/20100101 Firefox/135.0",
            )
            .await?
        }
    };

    let mut f = fs::File::create(&tmp_file).await.path(&tmp_file)?;
    f.write_all(&bytes).await.path(&tmp_file)?;
    f.flush().await.path(&tmp_file)?;
    f.sync_all().await.path(&tmp_file)?;

    fs::rename(&tmp_file, &cache_file).await.path(&cache_file)?;

    Ok(bytes)
}

fn hash(url: &str) -> String {
    let mut hasher = Sha256::default();
    hasher.update(url.as_bytes());
    format!("{:x}", hasher.finalize())
}
