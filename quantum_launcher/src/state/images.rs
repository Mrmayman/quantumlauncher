use std::{
    collections::{HashMap, HashSet},
    sync::Mutex,
};

use iced::{widget, Task};

use crate::{menu_renderer::Element, state::Message};

#[derive(Default)]
pub struct ImageState {
    bitmap: HashMap<String, widget::image::Handle>,
    svg: HashMap<String, widget::svg::Handle>,
    downloads_in_progress: HashSet<String>,
    /// A queue to request that an image be loaded.
    /// The `bool` represents whether it's a small
    /// icon or not.
    to_load: Mutex<HashSet<String>>,
}

impl ImageState {
    pub fn insert_image(&mut self, image: ql_mod_manager::store::ImageResult) {
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

        for url in self.to_load.lock().unwrap().drain() {
            if url.is_empty() {
                continue;
            }
            if self.downloads_in_progress.insert(url.clone()) {
                commands.push(Task::perform(
                    ql_mod_manager::store::download_image(url.clone()),
                    Message::CoreImageDownloaded,
                ));
            }
        }

        commands
    }

    pub fn queue(&mut self, url: &str) {
        let mut to_load = self.to_load.lock().unwrap();
        if !to_load.contains(url) {
            to_load.insert(url.to_owned());
        }
    }

    pub fn view<'a>(&self, url: Option<&str>, w: Option<f32>, h: Option<f32>) -> Element<'a> {
        let Some(url) = url else {
            return widget::Column::new().into();
        };
        if let Some(handle) = self.bitmap.get(url) {
            let mut e = widget::image(handle.clone()).content_fit(iced::ContentFit::ScaleDown);
            if let Some(s) = w {
                e = e.width(s);
            }
            if let Some(s) = h {
                e = e.height(s);
            }
            e.into()
        } else if let Some(handle) = self.svg.get(url) {
            let mut e = widget::svg(handle.clone());
            if let Some(s) = w {
                e = e.width(s);
            }
            if let Some(s) = h {
                e = e.height(s);
            }
            e.into()
        } else {
            let mut to_load = self.to_load.lock().unwrap();
            to_load.insert(url.to_owned());
            widget::Column::new().into()
        }
    }

    pub fn view_bitmap<'a>(
        &self,
        url: &str,
        w: Option<f32>,
        h: Option<f32>,
        fallback: Element<'a>,
    ) -> Element<'a> {
        if let Some(handle) = self.bitmap.get(url) {
            let mut e = widget::image(handle.clone());
            if let Some(s) = w {
                e = e.width(s);
            }
            if let Some(s) = h {
                e = e.height(s);
            }
            e.into()
        } else if self.svg.contains_key(url) {
            fallback
        } else {
            let mut to_load = self.to_load.lock().unwrap();
            to_load.insert(url.to_owned());
            fallback
        }
    }
}
