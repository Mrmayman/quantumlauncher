//! Persisted metadata for games that outlive the launcher.

use std::sync::{Arc, LazyLock, Mutex};

use chrono::{DateTime, Utc};
use ql_core::{Instance, InstanceKind, LAUNCHER_DIR};
use serde::{Deserialize, Serialize};
use sysinfo::{Pid, System};

const FILE_NAME: &str = "launched_games.json";

static SAVE_LOCK: LazyLock<tokio::sync::Mutex<()>> = LazyLock::new(|| tokio::sync::Mutex::new(()));

pub type SharedLaunchedGames = Arc<Mutex<Vec<LaunchedGame>>>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LaunchedGame {
    pub instance_name: String,
    pub instance_kind: InstanceKind,
    pub pid: u32,
    pub launched_at: DateTime<Utc>,
}

impl LaunchedGame {
    pub fn new(instance: &Instance, pid: u32) -> Self {
        Self {
            instance_name: instance.get_name().to_owned(),
            instance_kind: instance.kind,
            pid,
            launched_at: Utc::now(),
        }
    }

    pub fn instance(&self) -> Instance {
        Instance::new(&self.instance_name, self.instance_kind)
    }
}

pub fn shared() -> SharedLaunchedGames {
    Arc::new(Mutex::new(Vec::new()))
}

pub async fn load() -> Result<Vec<LaunchedGame>, String> {
    let path = LAUNCHER_DIR.join(FILE_NAME);
    let contents = match tokio::fs::read_to_string(&path).await {
        Ok(contents) => contents,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(format!("Could not read {}: {err}", path.display())),
    };

    let games = serde_json::from_str::<Vec<LaunchedGame>>(&contents)
        .map_err(|err| format!("Could not parse {}: {err}", path.display()))?;

    tokio::task::spawn_blocking(move || {
        let system = System::new_all().map_err(|err| err.to_string())?;
        Ok(games
            .into_iter()
            .filter(|game: &LaunchedGame| system.process(Pid::from_u32(game.pid)).is_some())
            .collect())
    })
    .await
    .map_err(|err| err.to_string())?
}

pub async fn save(games: SharedLaunchedGames) -> Result<(), String> {
    let _save_lock = SAVE_LOCK.lock().await;
    let games = games.lock().unwrap().clone();
    let contents = serde_json::to_string(&games).map_err(|err| err.to_string())?;
    let path = LAUNCHER_DIR.join(FILE_NAME);

    tokio::fs::write(&path, contents)
        .await
        .map_err(|err| format!("Could not write {}: {err}", path.display()))
}

pub async fn closed_instances(processes: Vec<(Instance, u32)>) -> Result<Vec<Instance>, String> {
    tokio::task::spawn_blocking(move || {
        let system = System::new_all().map_err(|err| err.to_string())?;
        Ok(processes
            .into_iter()
            .filter_map(|(instance, pid)| {
                system
                    .process(Pid::from_u32(pid))
                    .is_none()
                    .then_some(instance)
            })
            .collect())
    })
    .await
    .map_err(|err| err.to_string())?
}

pub async fn kill(pid: u32) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let system = System::new_all().map_err(|err| err.to_string())?;
        let Some(process) = system.process(Pid::from_u32(pid)) else {
            return Ok(());
        };
        if process.kill() {
            Ok(())
        } else {
            Err(format!("Could not kill process {pid}"))
        }
    })
    .await
    .map_err(|err| err.to_string())?
}
