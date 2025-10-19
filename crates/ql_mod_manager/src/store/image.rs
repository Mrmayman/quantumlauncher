use ql_core::{url_cache_get, IntoStringError};

#[derive(Clone)]
pub struct ImageResult {
    pub url: String,
    pub image: Vec<u8>,
    pub is_svg: bool,
}

impl std::fmt::Debug for ImageResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImageResult")
            .field("url", &self.url)
            .field("image", &format_args!("{} bytes", self.image.len()))
            .field("is_svg", &self.is_svg)
            .finish()
    }
}

pub async fn download_image(url: String) -> Result<ImageResult, String> {
    if url.starts_with("https://cdn.modrinth.com/") {
        // Does Modrinth CDN have a rate limit like their API?
        // I have no idea but from my testing it doesn't seem like they do.

        // let _lock = ql_instances::RATE_LIMITER.lock().await;
    }
    if url.is_empty() {
        return Err("url is empty".to_owned());
    }

    let image = url_cache_get(&url).await.strerr()?;
    let is_svg = image.starts_with(b"<svg") || url.to_lowercase().ends_with(".svg");

    Ok(ImageResult { url, image, is_svg })
}
