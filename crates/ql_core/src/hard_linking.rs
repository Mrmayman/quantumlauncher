use std::path::Path;

use crate::{IoError, file_utils::create_symlink_async};

async fn create_hard_links(links: Vec<(&Path, &Path)>) -> Result<(), IoError> {
    for (from, to) in links {
        create_symlink_async(from, to).await?;
    }
    Ok(())
}

pub async fn hard_link_files(
    sources: Vec<&Path>,
    destinations: Vec<&Path>
    ){

    let links: Vec<(&Path, &Path)> = sources
        .iter()
        .zip(destinations.iter())
        .map(|(&src, &dst)| (src, dst))
        .collect();

    create_hard_links(links)
        .await
        .unwrap();
}
