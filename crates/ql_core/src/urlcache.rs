use sha2::{Digest, Sha256};
use tokio::{fs, io::AsyncWriteExt};

use crate::{DownloadFileError, IntoIoError, LAUNCHER_DIR, download, file_utils};

pub async fn get(url: &str) -> Result<Vec<u8>, DownloadFileError> {
    get_ext(url, |n| n).await
}

pub async fn get_ext(
    url: &str,
    transform: impl FnOnce(Vec<u8>) -> Vec<u8>,
) -> Result<Vec<u8>, DownloadFileError> {
    let hash = hash(url);

    let cache_dir = LAUNCHER_DIR.join("downloads/cache");
    fs::create_dir_all(&cache_dir).await.path(&cache_dir)?;

    let cache_file = cache_dir.join(&hash);

    match fs::read(&cache_file).await {
        Ok(n) => return Ok(n),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.path(&cache_file).into()),
    }

    let bytes = match file_utils::download_file_to_bytes(url, true).await {
        Ok(n) => n,
        Err(_) => {
            // WTF: Some pesky cloud provider might be
            // blocking the launcher because they think it's a bot.
            //
            // I understand people do this to protect
            // their servers but what this is doing is clearly
            // not malicious. We're just downloading some images :)
            download(url).user_agent_spoof().bytes().await?
        }
    };
    let bytes = transform(bytes);

    let tmp_file = cache_dir.join(format!(".temp-{hash}"));
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
