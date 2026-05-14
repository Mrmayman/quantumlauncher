use crate::{DownloadFileError, request::download_asset};

pub async fn get(url: &str) -> Result<Vec<u8>, DownloadFileError> {
    get_ext(url, |n| n).await
}

pub async fn get_ext(
    url: &str,
    transform: impl FnOnce(Vec<u8>) -> Vec<u8>,
) -> Result<Vec<u8>, DownloadFileError> {
    let download_with_agent = download_asset(url, false).bytes().await;
    let bytes = match download_with_agent {
        Ok(n) => n,
        Err(_) => {
            // WTF: Some pesky cloud provider might be
            // blocking the launcher because they think it's a bot.
            //
            // I understand people do this to protect
            // their servers but what this is doing is clearly
            // not malicious. We're just downloading some images :)
            download_asset(url, true).bytes().await?
        }
    };
    let bytes = transform(bytes);

    Ok(bytes)
}
