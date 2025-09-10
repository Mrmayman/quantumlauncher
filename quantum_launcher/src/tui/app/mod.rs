// TUI App module root - re-exports state and core types

mod accounts_ctrl;
mod config_ctrl;
mod create_ctrl;
mod events;
mod instance_settings_ctrl;
mod instances_ctrl;
mod launch_ctrl;
pub(crate) mod licenses;
pub(crate) mod logs;
mod logs_ctrl;
mod settings_ctrl;
mod state;

pub use state::{
    AccountType, AddAccountFieldFocus, App, AppResult, ArgsEditKind, Instance,
    InstanceSettingsPage, InstanceSettingsTab, SettingsFocus, TabId, VersionCategory,
};

// Internal modules; callers should use App methods or import modules explicitly.
