use std::{
    collections::BTreeMap,
    io::Cursor,
    path::{Path, PathBuf},
    sync::Mutex,
};

use cfg_if::cfg_if;
use owo_colors::OwoColorize;
use ql_core::{
    constants::*,
    do_jobs, err, file_utils, info,
    json::{
        version::{
            Library, LibraryClassifier, LibraryDownloadArtifact, LibraryDownloads, LibraryExtract,
        },
        VersionDetails,
    },
    pt, DownloadProgress, IntoIoError, IoError,
};
use tokio::fs;

use super::{DownloadError, GameDownloader};

const MACOS_ARM_LWJGL_294_1: &str = "https://libraries.minecraft.net/org/lwjgl/lwjgl/lwjgl-platform/2.9.4-nightly-20150209/lwjgl-platform-2.9.4-nightly-20150209-natives-osx.jar";
const MACOS_ARM_LWJGL_294_2: &str = "https://github.com/Dungeons-Guide/lwjgl/releases/download/2.9.4-20150209-mmachina.2-syeyoung.1/lwjgl-platform-2.9.4-nightly-20150209-natives-osx-arm64.jar";

impl GameDownloader {
    pub async fn download_libraries(&mut self) -> Result<(), DownloadError> {
        info!("Downloading libraries");
        self.prepare_library_directories().await?;

        let total_libraries = self.version_json.libraries.len();
        let num_library = Mutex::new(0);

        let results = self
            .version_json
            .libraries
            .iter()
            .map(|lib| self.download_library_fn(lib, &num_library, total_libraries));

        // Uncomment for synchronous downloads. WAY slower,
        // but easier to debug/inspect logs of,
        // if you're working on the library downloader

        // for job in results {
        //     job.await?;
        // }

        // The one below is the concurrent downloader, downloading multiple
        // libraries at the same time. If you uncomment the above one, make sure
        // to comment this below one out.
        // This is WAY faster but harder to debug/inspect
        _ = do_jobs(results).await?;

        Ok(())
    }

    async fn download_library_fn(
        &self,
        library: &Library,
        library_i: &Mutex<usize>,
        library_len: usize,
    ) -> Result<(), DownloadError> {
        if !library.is_allowed() {
            pt!("{} {library:?}", "Skipping".underline());
            return Ok(());
        }

        self.download_library(library, None).await?;

        {
            let mut library_i = library_i.lock().unwrap();
            self.send_progress(
                DownloadProgress::DownloadingLibraries {
                    progress: *library_i,
                    out_of: library_len,
                },
                true,
            );
            *library_i += 1;
        }

        Ok(())
    }

    async fn prepare_library_directories(&self) -> Result<(), IoError> {
        let library_path = self.instance_dir.join("libraries");
        fs::create_dir_all(&library_path)
            .await
            .path(&library_path)?;
        let natives_path = library_path.join("natives");
        fs::create_dir_all(&natives_path).await.path(natives_path)?;
        Ok(())
    }

    pub async fn download_library(
        &self,
        library: &Library,
        artifact_fallback: Option<&LibraryDownloadArtifact>,
    ) -> Result<(), DownloadError> {
        let libraries_dir = self.instance_dir.join("libraries");

        if let Some(LibraryDownloads {
            artifact,
            classifiers,
            ..
        }) = library.downloads.as_ref()
        {
            if let Some(artifact) = artifact.as_ref().or(artifact_fallback) {
                self.download_library_artifact(
                    library,
                    &libraries_dir,
                    artifact,
                    classifiers.as_ref(),
                )
                .await?;
            }
            if let Some(classifiers) = classifiers {
                self.download_library_native(classifiers, &libraries_dir, library.extract.as_ref())
                    .await?;
            }
        } else if let Some(artifact) = artifact_fallback {
            self.download_library_artifact(library, &libraries_dir, artifact, None)
                .await?;
        }

        Ok(())
    }

    async fn download_library_artifact(
        &self,
        library: &Library,
        libraries_dir: &Path,
        artifact: &LibraryDownloadArtifact,
        classifiers: Option<&BTreeMap<String, LibraryClassifier>>,
    ) -> Result<(), DownloadError> {
        pt!(
            "{} {}:\n  {}",
            "Downloading".underline(),
            library.name.as_deref().unwrap_or_default(),
            artifact.url.bright_black()
        );
        let jar_file = self
            .download_library_normal(artifact, libraries_dir)
            .await?;

        let natives_path = self.instance_dir.join("libraries/natives");
        self.extractlib_natives_field(library, classifiers, jar_file, &natives_path, artifact)
            .await?;
        self.extractlib_name_natives(library, artifact).await?;
        Ok(())
    }

    /// Simplified function to extract native libraries.
    ///
    /// This is only used to migrate from QuantumLauncher
    /// v0.1/0.2 to 0.3 or above.
    ///
    /// This function only supports Windows and Linux for x86_64
    /// since it doesn't have special library handling logic for
    /// other platforms, because the old versions being migrated from
    /// didn't support other platforms in the first place.
    ///
    /// For "real" library downloading when creating an instance
    /// see [`GameDownloader::download_library_fn`]
    #[allow(clippy::doc_markdown)]
    pub async fn migrate_extract_native_library(
        instance_dir: &Path,
        library: &Library,
        jar_file: Vec<u8>,
        artifact: &LibraryDownloadArtifact,
    ) -> Result<(), DownloadError> {
        let d = GameDownloader::with_existing_instance(
            VersionDetails::load_from_path(instance_dir).await?,
            instance_dir.to_owned(),
            None,
        );
        let natives_path = instance_dir.join("libraries/natives");

        // Why 2 functions? Because unfortunately there are multiple formats
        // natives can come in, and we need to support all of them.
        d.extractlib_natives_field(
            library,
            Some(&BTreeMap::new()),
            jar_file,
            &natives_path,
            artifact,
        )
        .await?;

        d.extractlib_name_natives(library, artifact).await?;

        Ok(())
    }

    async fn download_library_normal(
        &self,
        artifact: &LibraryDownloadArtifact,
        libraries_dir: &Path,
    ) -> Result<Vec<u8>, DownloadError> {
        let lib_file_path = libraries_dir.join(PathBuf::from(artifact.get_path()));

        let lib_dir_path = lib_file_path
            .parent()
            .expect(
                "Downloaded java library does not have parent module like the sun in com.sun.java",
            )
            .to_path_buf();

        fs::create_dir_all(&lib_dir_path).await.path(lib_dir_path)?;
        let library_downloaded = file_utils::download_file_to_bytes(&artifact.url, false).await?;

        fs::write(&lib_file_path, &library_downloaded)
            .await
            .path(lib_file_path)?;

        Ok(library_downloaded)
    }

    async fn download_library_native(
        &self,
        classifiers: &BTreeMap<String, LibraryClassifier>,
        libraries_dir: &Path,
        extract: Option<&LibraryExtract>,
    ) -> Result<(), DownloadError> {
        let natives_dir = libraries_dir.join("natives");

        for (os, download) in classifiers {
            if os == "sources" {
                continue;
            }
            #[allow(unused)]
            #[allow(clippy::let_and_return)]
            if !(OS_NAMES.iter().any(|os_name| {
                let os_name = format!("natives-{os_name}");
                cfg_if!(if #[cfg(feature = "simulate_linux_arm64")] {
                    // Simulating Linux ARM 64
                    let matches = os == "natives-linux-arm64"
                        || (*os == os_name && download.url.contains("arm64"));
                } else if #[cfg(feature = "simulate_macos_arm64")] {
                    // Simulating macOS ARM 64
                    let matches = os == "natives-osx-arm64";
                } else if #[cfg(feature = "simulate_linux_arm32")] {
                    // Simulating Linux ARM 32
                    let matches = os == "natives-linux-arm32"
                        || (*os == os_name && download.url.contains("arm32"));
                } else if #[cfg(all(target_os = "macos", target_arch = "aarch64"))] {
                    // macOS ARM 64
                    let matches = os == "natives-osx-arm64";
                } else if #[cfg(all(target_os = "linux", target_arch = "aarch64"))] {
                    // Linux ARM 64
                    let matches = os == "natives-linux-arm64"
                        || (*os == os_name && download.url.contains("arm64"));
                } else if #[cfg(all(target_os = "windows", target_arch = "x86"))] {
                    // Windows x86 32-bit
                    let matches = os == "natives-windows-32";
                } else if #[cfg(all(target_os = "windows", target_arch = "x86_64"))] {
                    // Windows x86_64
                    let matches = (os == "natives-windows-64") || (os == "natives-windows");
                } else if #[cfg(all(target_os = "linux", target_arch = "arm"))] {
                    // Linux ARM 32
                    let matches = os == "natives-linux-arm32"
                        || (*os == os_name && download.url.contains("arm32"));
                } else {
                    // Others
                    let matches = *os == os_name;
                });

                matches
            })) {
                pt!("  {} {os}", "Skipping".bright_black());
                continue;
            }

            pt!(
                "  Natives ({}):\n    {}",
                "4: classifiers".blue(),
                download.url.bright_black()
            );
            self.extract_file(download.url.clone()).await?;
        }

        if let Some(extract) = extract {
            for exclusion in &extract.exclude {
                let path = natives_dir.join(exclusion);

                if !path.starts_with(&natives_dir) {
                    return Err(DownloadError::NativesOutsideDirRemove);
                }

                if let Ok(meta) = fs::metadata(&path).await {
                    if meta.is_dir() {
                        fs::remove_dir_all(&path).await.path(path)?;
                    } else {
                        fs::remove_file(&path).await.path(path)?;
                    }
                }
            }
        }

        Ok(())
    }

    async fn extract_file(&self, mut url: String) -> Result<(), DownloadError> {
        if url == "https://github.com/theofficialgman/lwjgl3-binaries-arm64/raw/lwjgl-3.1.6/lwjgl-jemalloc-natives-linux.jar" {
            url = "https://github.com/theofficialgman/lwjgl3-binaries-arm64/raw/lwjgl-3.1.6/lwjgl-jemalloc-patched-natives-linux-arm64.jar".to_owned();
        }
        if (cfg!(target_arch = "aarch64") && url == MACOS_ARM_LWJGL_294_1) || url == "https://github.com/MinecraftMachina/lwjgl/releases/download/2.9.4-20150209-mmachina.2/lwjgl-platform-2.9.4-nightly-20150209-natives-osx.jar" {
            url = MACOS_ARM_LWJGL_294_2.to_owned();
        }

        #[cfg(any(
            feature = "simulate_linux_arm64",
            all(target_os = "linux", target_arch = "aarch64")
        ))]
        if url.ends_with("lwjgl-core-natives-linux.jar") {
            url = url.replace(
                "lwjgl-core-natives-linux.jar",
                "lwjgl-natives-linux-arm64.jar",
            );
        }

        if !self
            .already_downloaded_natives
            .lock()
            .await
            .insert(url.clone())
        {
            return Ok(());
        }
        let file_bytes = match file_utils::download_file_to_bytes(&url, false).await {
            Ok(n) => n,
            #[cfg(any(
                all(target_os = "linux", target_arch = "aarch64"),
                feature = "simulate_linux_arm64"
            ))]
            Err(ql_core::RequestError::DownloadError { code, .. }) if code.as_u16() == 404 => {
                file_utils::download_file_to_bytes(
                    &url.replace("linux.jar", "linux-arm64.jar"),
                    false,
                )
                .await?
            }
            Err(err) => Err(err)?,
        };

        let extract_path = self.instance_dir.join("libraries/natives");
        file_utils::extract_zip_archive(Cursor::new(file_bytes), &extract_path, true)
            .await
            .map_err(DownloadError::NativesExtractError)?;
        Ok(())
    }

    async fn extractlib_natives_field(
        &self,
        library: &Library,
        classifiers: Option<&BTreeMap<String, LibraryClassifier>>,
        jar_file: Vec<u8>,
        natives_path: &Path,
        artifact: &LibraryDownloadArtifact,
    ) -> Result<(), DownloadError> {
        let name = library.name.as_deref().unwrap_or_default();

        let Some(natives) = &library.natives else {
            return Ok(());
        };

        cfg_if!(
            if #[cfg(any(
                target_arch = "aarch64",
                target_arch = "arm",
                target_arch = "x86",
                feature = "simulate_linux_arm64",
                feature = "simulate_macos_arm64",
                feature = "simulate_linux_arm32",
            ))] {
                let Some(natives_name) = natives.get(&format!("{OS_NAME}-{ARCH}")) else {
                    return Ok(());
                };
            } else {
                let Some(natives_name) = natives.get(OS_NAME) else {
                    return Ok(());
                };
            }
        );

        if library
            .name
            .as_deref()
            .is_none_or(|n| n != "ca.weblite:java-objc-bridge:1.0.0")
        {
            // TODO: Somehow obtain aarch64 natives for ca.weblite:java-objc-bridge:1.0.0
            // Bridge 1.1 has them but 1.0 doesn't
            pt!(
                "  Natives ({}): {}",
                "1: main jar".cyan(),
                name.bright_black()
            );

            if let Err(err) =
                file_utils::extract_zip_archive(Cursor::new(jar_file), natives_path, true).await
            {
                err!("Couldn't extract main jar: {err}");
            }
        }

        let natives_url = if let Some(natives) = classifiers.and_then(|n| n.get(natives_name)) {
            natives.url.clone()
        } else {
            let url = &artifact.url[..artifact.url.len() - 4];
            format!("{url}-{natives_name}.jar")
        };

        pt!(
            "  Natives ({}): {}\n    {}",
            "2: .natives".purple(),
            name.bright_black(),
            natives_url.bright_black()
        );
        self.extract_file(natives_url).await?;

        Ok(())
    }

    async fn extractlib_name_natives(
        &self,
        library: &Library,
        artifact: &LibraryDownloadArtifact,
    ) -> Result<(), DownloadError> {
        let Some(name) = &library.name else {
            return Ok(());
        };

        if !name.contains("native") {
            return Ok(());
        }

        cfg_if!(if #[cfg(any(
            target_arch = "aarch64",
            feature = "simulate_linux_arm64",
            feature = "simulate_macos_arm64"
        ))] {
            let is_compatible = name.contains("aarch") || name.contains("arm64");
        } else if #[cfg(feature = "simulate_linux_arm32")] {
            let is_compatible = name.contains("arm32");
        } else if #[cfg(target_arch = "aarch64")] {
            let is_compatible = name.contains("aarch") || name.contains("arm64");
        } else if #[cfg(target_arch = "arm")] {
            let is_compatible = name.contains("arm32");
        } else if #[cfg(target_arch = "x86")] {
            let is_compatible = name.contains("x86") && !name.contains("x86_64");
        } else {
            let is_compatible = !(name.contains("aarch")
                || name.contains("arm")
                || (name.contains("x86") && !name.contains("x86_64")));
        });

        if is_compatible {
            pt!(
                "  Natives ({}): {}",
                "3: based on name".yellow(),
                name.bright_black()
            );
            self.extract_file(artifact.url.clone()).await?;
        }

        Ok(())
    }
}
