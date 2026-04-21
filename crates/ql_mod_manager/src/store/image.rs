use image::{ImageFormat, imageops::FilterType};
use ql_core::{IntoStringError, urlcache};
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

    let image = urlcache::get(&url).await.strerr()?;
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

    let image = urlcache::get_ext(&url, |bytes| {
        is_svg |= bytes.starts_with(b"<svg");
        let is_gif = bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a");

        if is_svg || is_gif {
            return bytes;
        }
        resize_to_icon(&bytes).unwrap_or(bytes)
    })
    .await
    .strerr()?;

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
    let mut buf = Vec::with_capacity(32 * 32 * 4 + 64);
    resized
        .write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)
        .ok()?;
    Some(buf)
}
