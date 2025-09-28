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

pub async fn download_image(url: String, icon: bool) -> Result<ImageResult, String> {
    if url.starts_with("https://cdn.modrinth.com/") {
        // Does Modrinth CDN have a rate limit like their API?
        // I have no idea but from my testing it doesn't seem like they do.

        // let _lock = ql_instances::RATE_LIMITER.lock().await;
    }
    if url.is_empty() {
        return Err("url is empty".to_owned());
    }

    let image = url_cache_get(&url).await.strerr()?;

    if url.to_lowercase().ends_with(".svg") {
        return Ok(ImageResult {
            url,
            image,
            is_svg: true,
        });
    }

    if let Ok(text) = std::str::from_utf8(&image) {
        if text.starts_with("<svg") {
            return Ok(ImageResult {
                url,
                image,
                is_svg: true,
            });
        }
    }

    let img = image::io::Reader::new(std::io::Cursor::new(image))
        .with_guessed_format()
        .map_err(|err| format!("{url}: {err}"))?
        .decode()
        .map_err(|err| format!("{url}: {err}"))?;

    let img = img.thumbnail(if icon { 32 } else { 1000 }, 426);
    // let img = img.resize(32, 32, image::imageops::FilterType::Nearest);

    let mut buffer = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut buffer);
    img.write_to(&mut cursor, image::ImageFormat::Png)
        .map_err(|err| format!("{url}: {err}"))?;

    Ok(ImageResult {
        url,
        image: buffer,
        is_svg: false,
    })
}
