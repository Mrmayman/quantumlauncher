pub mod launch;
pub mod list_versions;
mod migrate;

pub mod notes {
    use ql_core::{InstanceSelection, IntoIoError, IoError};

    pub async fn read(instance: InstanceSelection) -> Result<String, IoError> {
        let path = instance.get_instance_path().join("notes.md");
        match tokio::fs::read_to_string(&path).await {
            Ok(contents) => Ok(contents),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
            Err(e) => Err(e).path(&path),
        }
    }

    pub async fn write(instance: InstanceSelection, notes: String) -> Result<(), IoError> {
        let path = instance.get_instance_path().join("notes.md");
        tokio::fs::write(&path, &notes).await.path(&path)
    }
}
