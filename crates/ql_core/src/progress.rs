use std::{fmt::Display, sync::Arc};

use sipper::Sipper;

/// An enum representing the progress in downloading
/// a Minecraft instance.
///
/// # Order
/// 1) Manifest Json
/// 2) Version Json
/// 3) Logging config
/// 4) Jar
/// 5) Libraries
/// 6) Assets
#[derive(Debug, Clone, Copy, Default)]
pub enum DownloadProgress {
    #[default]
    DownloadingJsonManifest,
    DownloadingVersionJson,
    DownloadingAssets {
        progress: usize,
        out_of: usize,
    },
    DownloadingLibraries {
        progress: usize,
        out_of: usize,
    },
    DownloadingJar,
}

impl DownloadProgress {
    pub const TOTAL: f32 = 10.0;
}

impl Display for DownloadProgress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DownloadProgress::DownloadingJsonManifest => write!(f, "Downloading Manifest JSON"),
            DownloadProgress::DownloadingVersionJson => write!(f, "Downloading Version JSON"),
            DownloadProgress::DownloadingAssets { progress, out_of } => {
                write!(f, "Downloading asset {progress} / {out_of}")
            }
            DownloadProgress::DownloadingLibraries { progress, out_of } => {
                write!(f, "Downloading library {progress} / {out_of}")
            }
            DownloadProgress::DownloadingJar => write!(f, "Downloading Game Jar file"),
        }
    }
}

impl From<&DownloadProgress> for f32 {
    fn from(val: &DownloadProgress) -> Self {
        match val {
            DownloadProgress::DownloadingJsonManifest => 0.1,
            DownloadProgress::DownloadingVersionJson => 0.2,
            DownloadProgress::DownloadingJar => 0.3,
            DownloadProgress::DownloadingLibraries { progress, out_of } => {
                (*progress as f32 / *out_of as f32) + 1.0
            }
            DownloadProgress::DownloadingAssets { progress, out_of } => {
                (*progress as f32 * 8.0 / *out_of as f32) + 2.0
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct GenericProgress {
    pub done: usize,
    pub total: usize,
    pub message: Option<String>,
    pub has_finished: bool,
}

impl Default for GenericProgress {
    fn default() -> Self {
        Self {
            done: 0,
            total: 1,
            message: None,
            has_finished: false,
        }
    }
}

impl GenericProgress {
    #[must_use]
    pub fn finished() -> Self {
        Self {
            has_finished: true,
            done: 1,
            total: 1,
            message: None,
        }
    }
}

pub trait Progress: std::fmt::Debug + Send + Sync {
    fn get_num(&self) -> f32;
    fn get_message(&self) -> Option<String>;
    fn total(&self) -> f32;

    fn generic(&self) -> GenericProgress {
        let done = (self.get_num() * 100.0) as usize;
        let total = (self.total() * 100.0) as usize;
        let message = self.get_message();

        GenericProgress {
            done,
            total,
            message,
            has_finished: false,
        }
    }
}

impl Progress for DownloadProgress {
    fn get_num(&self) -> f32 {
        f32::from(self)
    }

    fn get_message(&self) -> Option<String> {
        Some(self.to_string())
    }

    fn total(&self) -> f32 {
        Self::TOTAL
    }
}

impl Progress for GenericProgress {
    fn get_num(&self) -> f32 {
        self.done as f32 / self.total as f32
    }

    fn get_message(&self) -> Option<String> {
        self.message.clone()
    }

    fn total(&self) -> f32 {
        1.0
    }

    fn generic(&self) -> GenericProgress {
        self.clone()
    }
}

impl<T: Send + Sync + std::fmt::Debug + Progress> Progress for Box<T> {
    fn get_num(&self) -> f32 {
        self.as_ref().get_num()
    }

    fn get_message(&self) -> Option<String> {
        self.as_ref().get_message()
    }

    fn total(&self) -> f32 {
        self.as_ref().total()
    }

    fn generic(&self) -> GenericProgress {
        self.as_ref().generic()
    }
}

impl Progress for Box<dyn Progress> {
    fn get_num(&self) -> f32 {
        self.as_ref().get_num()
    }

    fn get_message(&self) -> Option<String> {
        self.as_ref().get_message()
    }

    fn total(&self) -> f32 {
        self.as_ref().total()
    }

    fn generic(&self) -> GenericProgress {
        self.as_ref().generic()
    }
}

impl Progress for Arc<dyn Progress> {
    fn get_num(&self) -> f32 {
        self.as_ref().get_num()
    }

    fn get_message(&self) -> Option<String> {
        self.as_ref().get_message()
    }

    fn total(&self) -> f32 {
        self.as_ref().total()
    }

    fn generic(&self) -> GenericProgress {
        self.as_ref().generic()
    }
}

pub async fn pipe_progress<P: Progress, Fut, F, Out>(
    sender: Option<impl Into<sipper::Sender<GenericProgress>>>,
    f: F,
) -> Out
where
    F: FnOnce(Option<sipper::Sender<P>>) -> Fut,
    Fut: std::future::Future<Output = Out>,
{
    if let Some(sender) = sender {
        sipper::sipper(|s| f(Some(s)))
            .with(|n: P| n.generic())
            .run(sender)
            .await
    } else {
        f(None).await
    }
}

pub async fn pipe_progress_ext<P: Progress, P2, Fut, F, Out>(
    sender: Option<impl Into<sipper::Sender<P2>>>,
    f: F,
    convert: impl FnMut(P) -> P2,
) -> Out
where
    F: FnOnce(Option<sipper::Sender<P>>) -> Fut,
    Fut: std::future::Future<Output = Out>,
{
    if let Some(sender) = sender {
        sipper::sipper(|s| f(Some(s)))
            .with(convert)
            .run(sender)
            .await
    } else {
        f(None).await
    }
}
