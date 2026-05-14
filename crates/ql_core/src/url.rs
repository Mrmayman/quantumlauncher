use crate::{DownloadFileError, download, file_utils};

pub async fn get(url: &str) -> Result<Vec<u8>, DownloadFileError> {
    get_ext(url, |n| n).await
}

pub async fn get_ext(
    url: &str,
    transform: impl FnOnce(Vec<u8>) -> Vec<u8>,
) -> Result<Vec<u8>, DownloadFileError> {
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

    Ok(bytes)
}
