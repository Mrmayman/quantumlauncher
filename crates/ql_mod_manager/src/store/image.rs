use image::{ImageFormat, imageops::FilterType};
use ql_core::{IntoStringError, RequestError, download};
use std::io::Cursor;

#[derive(Clone)]
pub struct Output {
    pub url: String,
    pub image: Vec<u8>,
    pub is_svg: bool,
}

impl std::fmt::Debug for Output {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImageResult")
            .field("url", &self.url)
            .field("image", &format_args!("{} bytes", self.image.len()))
            .field("is_svg", &self.is_svg)
            .finish()
    }
}

/// Downloads full-scale images.
///
/// See [`get_icon`] if you just want icons,
/// as it scales them down for efficiency.
pub async fn get(url: String) -> Result<Output, String> {
    if url.is_empty() {
        return Err("url is empty".to_owned());
    }

    let image = download_icon(&url).await.strerr()?;
    let is_svg = image.starts_with(b"<svg") || url.to_lowercase().ends_with(".svg");

    Ok(Output { url, image, is_svg })
}

pub const ICON_SIZE: u32 = 40;
pub const ICON_SIZE_F32: f32 = 40.0;

/// Downloads icons (cached), and scales them down to 64x64 for efficiency.
pub async fn get_icon(url: String) -> Result<Output, String> {
    if url.is_empty() {
        return Err("url is empty".to_owned());
    }

    let mut is_svg = url.to_lowercase().ends_with(".svg");

    let image = {
        let bytes = download_icon(&url).await.strerr()?;
        is_svg |= bytes.starts_with(b"<svg");
        let is_gif = bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a");

        if is_svg || is_gif {
            bytes
        } else {
            resize_to_icon(&bytes).unwrap_or(bytes)
        }
    };

    Ok(Output { url, image, is_svg })
}

fn resize_to_icon(bytes: &[u8]) -> Option<Vec<u8>> {
    let img = image::load_from_memory(bytes).ok()?;
    if img.width() <= ICON_SIZE && img.height() <= ICON_SIZE {
        // Skip if already small enough
        return None;
    }
    if img.width() != img.height() {
        // Uneven
        return None;
    }

    let resized = img.resize(ICON_SIZE, ICON_SIZE, FilterType::Triangle);
    let mut buf = Vec::with_capacity(ICON_SIZE as usize * ICON_SIZE as usize * 4);
    resized
        .write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)
        .ok()?;
    Some(buf)
}

async fn download_icon(url: &str) -> Result<Vec<u8>, RequestError> {
    let download_with_agent = download(url).bytes().await;
    Ok(match download_with_agent {
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
    })
}
