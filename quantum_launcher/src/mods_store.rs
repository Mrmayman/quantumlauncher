use std::collections::BTreeMap;
use std::{collections::HashMap, time::Instant};

use iced::futures::executor::block_on;
use iced::{widget::scrollable::AbsoluteOffset, Task};
use ql_core::{
    json::{instance_config::InstanceConfigJson, version::VersionDetails},
    InstanceSelection, IntoStringError, JsonFileError, Loader, StoreBackendType,
};
use ql_mod_manager::store::{ModIndex, Query, QueryType};

use crate::state::{InstallModsMessage, Launcher, MenuModsDownload, Message, State};

impl Launcher {
    pub fn open_mods_store(&mut self) -> Result<Task<Message>, JsonFileError> {
        let selection = self.instance();

        let config = block_on(InstanceConfigJson::read(selection))?;
        let version_json = if let State::EditMods(menu) = &self.state {
            menu.version_json.clone()
        } else {
            Box::new(block_on(VersionDetails::load(selection))?)
        };
        let mod_index = block_on(ModIndex::load(selection))?;

        let mut menu = MenuModsDownload {
            scroll_offset: AbsoluteOffset::default(),
            config,
            version_json,
            latest_load: Instant::now(),
            query: String::new(),
            results: None,
            opened_mod: None,
            mod_descriptions: HashMap::new(),
            mods_download_in_progress: BTreeMap::new(),
            mod_index,
            is_loading_continuation: false,
            has_continuation_ended: false,

            backend: StoreBackendType::Modrinth,
            query_type: QueryType::Mods,
        };
        let command = menu.search_store(
            matches!(&self.selected_instance, Some(InstanceSelection::Server(_))),
            0,
        );
        self.state = State::ModsDownload(menu);
        Ok(command)
    }
}

impl MenuModsDownload {
    pub fn search_store(&mut self, is_server: bool, offset: usize) -> Task<Message> {
        let loader = Loader::try_from(self.config.mod_type.as_str()).ok();

        let query = Query {
            name: self.query.clone(),
            version: self.version_json.get_id().to_owned(),
            loader,
            server_side: is_server,
            // open_source: false, // TODO: Add Open Source filter
        };
        let backend = self.backend;
        Task::perform(
            ql_mod_manager::store::search(query, offset, backend, self.query_type),
            |n| Message::InstallMods(InstallModsMessage::SearchResult(n.strerr())),
        )
    }
}
