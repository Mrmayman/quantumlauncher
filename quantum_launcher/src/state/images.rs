use std::{
    collections::{HashMap, HashSet},
    sync::Mutex,
};

use iced::{Task, widget};
use ql_mod_manager::store::image;

use crate::{menu_renderer::Element, state::Message};

macro_rules! sized {
    ( $e:expr, $w:expr, $h:expr ) => {{
        let mut e = $e;
        if let Some(s) = $w {
            e = e.width(s);
        }
        if let Some(s) = $h {
            e = e.height(s);
        }
        e.into()
    }};
}

#[derive(Default)]
pub struct ImageState {
    bitmap: HashMap<String, widget::image::Handle>,
    svg: HashMap<String, widget::svg::Handle>,
    downloads_in_progress: HashSet<String>,
    /// A queue to request that an image be loaded.
    ///
    /// The `bool` represents whether it's a small
    /// icon or not.
    to_load: Mutex<HashMap<String, bool>>,
}

impl ImageState {
    pub fn insert_image(&mut self, image: image::Output) {
        if image.is_svg {
            let handle = widget::svg::Handle::from_memory(image.image);
            self.svg.insert(image.url, handle);
        } else {
            self.bitmap
                .insert(image.url, widget::image::Handle::from_bytes(image.image));
        }
    }

    pub fn task_get_imgs_to_load(&mut self) -> Vec<Task<Message>> {
        let mut commands = Vec::new();

        // TODO: rewrite this to use do_jobs, may reduce memory usage
        for (url, is_icon) in self.to_load.lock().unwrap().drain() {
            if url.is_empty() {
                continue;
            }
            if self.downloads_in_progress.insert(url.clone()) {
                commands.push(if is_icon {
                    Task::perform(image::get_icon(url), Message::CoreImageDownloaded)
                } else {
                    Task::perform(image::get(url), Message::CoreImageDownloaded)
                });
            }
        }

        commands
    }

    pub fn queue(&mut self, url: &str, is_icon: bool) {
        let mut to_load = self.to_load.lock().unwrap();
        if !to_load.contains_key(url) {
            to_load.insert(url.to_owned(), is_icon);
        }
    }

    pub fn view<'a>(&self, url: Option<&str>, w: Option<f32>, h: Option<f32>) -> Element<'a> {
        let Some(url) = url else {
            return sized!(widget::Column::new(), w, h);
        };

        let is_small = |n| n <= image::ICON_SIZE_F32;
        let is_icon = w.is_some_and(is_small) && h.is_some_and(is_small);

        if let Some(handle) = self.bitmap.get(url) {
            sized!(
                widget::image(handle.clone()).content_fit(iced::ContentFit::ScaleDown),
                w,
                h
            )
        } else if let Some(handle) = self.svg.get(url) {
            sized!(widget::svg(handle.clone()), w, h)
        } else {
            let mut to_load = self.to_load.lock().unwrap();
            to_load.insert(url.to_owned(), is_icon);
            sized!(widget::Column::new(), w, h)
        }
    }
}
