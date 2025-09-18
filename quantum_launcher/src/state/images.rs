use std::{
    collections::{HashMap, HashSet},
    sync::Mutex,
};

use iced::{widget, Task};

use crate::{menu_renderer::Element, state::Message};

#[derive(Default)]
pub struct ImageState {
    bitmap: HashMap<String, iced::widget::image::Handle>,
    svg: HashMap<String, iced::widget::svg::Handle>,
    downloads_in_progress: HashSet<String>,
    /// A queue to request that an image be loaded.
    /// The `bool` represents whether it's a small
    /// icon or not.
    to_load: Mutex<HashMap<String, bool>>,
}

impl ImageState {
    pub fn insert_image(&mut self, image: ql_mod_manager::store::ImageResult) {
        if image.is_svg {
            let handle = iced::widget::svg::Handle::from_memory(image.image);
            self.svg.insert(image.url, handle);
        } else {
            self.bitmap.insert(
                image.url,
                iced::widget::image::Handle::from_bytes(image.image),
            );
        }
    }

    pub fn get_imgs_to_load(&mut self) -> Vec<Task<Message>> {
        let mut commands = Vec::new();

        let mut images_to_load = self.to_load.lock().unwrap();

        for (url, is_icon) in images_to_load.iter() {
            if !self.downloads_in_progress.contains(url) {
                self.downloads_in_progress.insert(url.to_owned());
                commands.push(Task::perform(
                    ql_mod_manager::store::download_image(url.to_owned(), *is_icon),
                    |n| Message::CoreImageDownloaded(n),
                ));
            }
        }

        images_to_load.clear();
        commands
    }

    pub fn view<'a>(&self, url: &str, size: Option<u16>, fallback: Element<'a>) -> Element<'a> {
        if let Some(handle) = self.bitmap.get(url) {
            let e = widget::image(handle.clone());
            if let Some(s) = size {
                e.width(s).height(s).into()
            } else {
                e.into()
            }
        } else if let Some(handle) = self.svg.get(url) {
            let e = widget::svg(handle.clone());
            if let Some(s) = size {
                e.width(s).height(s).into()
            } else {
                e.into()
            }
        } else {
            let mut to_load = self.to_load.lock().unwrap();
            to_load.insert(url.to_owned(), size.is_some());
            fallback
        }
    }
}
