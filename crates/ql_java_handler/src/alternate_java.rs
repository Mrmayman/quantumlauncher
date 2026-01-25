//! A module to install Java from various third party sources
//! (like Amazon Corretto) if Mojang doesn't provide Java for your specific platform.

use std::{io::Cursor, path::Path};

use cfg_if::cfg_if;
use ql_core::{file_utils, GenericProgress};
use sipper::Sender;

use crate::{extract_tar_gz, JavaInstallError, JavaVersion};

pub(crate) async fn install(
    version: JavaVersion,
    mut progress: Option<Sender<GenericProgress>>,
    install_dir: &Path,
) -> Result<(), JavaInstallError> {
    let url = version.get_alternate_url();

    let Some(url) = url else {
        return Err(error_unsupported(version));
    };

    if let Some(progress) = &mut progress {
        progress
            .send(GenericProgress {
                done: 0,
                total: 2,
                message: Some("Getting compressed archive".to_owned()),
                has_finished: false,
            })
            .await;
    }
    let file_bytes = file_utils::download_file_to_bytes(url, false).await?;
    if let Some(progress) = &mut progress {
        progress
            .send(GenericProgress {
                done: 1,
                total: 2,
                message: Some("Extracting archive".to_owned()),
                has_finished: false,
            })
            .await;
    }
    if url.ends_with("tar.gz") {
        extract_tar_gz(&file_bytes, install_dir).map_err(JavaInstallError::TarGzExtract)?;
    } else if url.ends_with("zip") {
        file_utils::extract_zip_archive(Cursor::new(file_bytes), &install_dir, true).await?;
    } else {
        return Err(JavaInstallError::UnknownExtension(url.to_owned()));
    }
    Ok(())
}

fn error_unsupported(version: JavaVersion) -> JavaInstallError {
    if let JavaVersion::Java16 | JavaVersion::Java17 | JavaVersion::Java21 = version {
        if JavaVersion::Java8.get_alternate_url().is_some() {
            JavaInstallError::UnsupportedOnlyJava8
        } else {
            JavaInstallError::UnsupportedPlatform
        }
    } else {
        JavaInstallError::UnsupportedPlatform
    }
}

impl JavaVersion {
    #[must_use]
    pub(crate) fn get_alternate_url(self) -> Option<&'static str> {
        // Sources:
        // https://aws.amazon.com/corretto/
        // https://github.com/Mrmayman/get-jdk/

        cfg_if!(if #[cfg(target_os = "linux")] {
            return self.get_url_linux();
        } else if #[cfg(target_os = "macos")] {
            return self.get_url_macos();
        } else if #[cfg(target_os = "windows")] {
            return self.get_url_windows();
        } else if #[cfg(all(target_os = "freebsd", target_arch = "x86_64"))] {
            // # Sourcing
            // The following is a re-packaged version of:
            // <https://pkg.freebsd.org/FreeBSD:13:amd64/quarterly/All/openjdk8-8.452.09.1_1.pkg>
            //
            // No modifications were made to Java itself,
            // it was simply re-archived with a different directory structure
            return matches!(self, JavaVersion::Java8).then_some(
                "https://github.com/Mrmayman/get-jdk/releases/download/java8-1/jdk-8u452-freebsd-x64.tar.gz"
            );
        } else if #[cfg(all(target_os = "solaris", target_arch = "x86_64"))] {
            return matches!(self, JavaVersion::Java8).then_some(
                "https://github.com/Mrmayman/get-jdk/releases/download/java8-1/jdk-8u231-solaris-x64.tar.gz"
            );
        } else if #[cfg(all(target_os = "solaris", target_arch = "sparc64"))] {
            return matches!(self, JavaVersion::Java8).then_some(
                "https://github.com/Mrmayman/get-jdk/releases/download/java8-1/jdk-8u231-solaris-sparcv9.tar.gz"
            );
        });
        #[allow(unreachable_code)]
        None
    }

    #[cfg(target_os = "linux")]
    fn get_url_linux(self) -> Option<&'static str> {
        cfg_if!(if #[cfg(all(target_env = "musl", target_arch = "x86_64"))] {
            return Some(match self {
                JavaVersion::Java16 |
                JavaVersion::Java17 => "https://corretto.aws/downloads/latest/amazon-corretto-17-x64-alpine-jdk.tar.gz",
                JavaVersion::Java21 => "https://corretto.aws/downloads/latest/amazon-corretto-21-x64-alpine-jdk.tar.gz",
                JavaVersion::Java25 => "https://corretto.aws/downloads/latest/amazon-corretto-25-x64-alpine-jdk.tar.gz"
                JavaVersion::Java8 => "https://corretto.aws/downloads/latest/amazon-corretto-8-x64-alpine-jdk.tar.gz",
            });
        } else if #[cfg(all(target_env = "musl", target_arch = "aarch64"))] {
            return Some(match self {
                JavaVersion::Java16 |
                JavaVersion::Java17 => "https://corretto.aws/downloads/latest/amazon-corretto-17-aarch64-alpine-jdk.tar.gz",
                JavaVersion::Java21 => "https://corretto.aws/downloads/latest/amazon-corretto-21-aarch64-alpine-jdk.tar.gz",
                JavaVersion::Java25 => "https://corretto.aws/downloads/latest/amazon-corretto-25-aarch64-alpine-jdk.tar.gz",
                JavaVersion::Java8 => "https://corretto.aws/downloads/latest/amazon-corretto-8-aarch64-alpine-jdk.tar.gz",
            });
        } else if #[cfg(target_arch = "x86_64")] {
            return Some(match self {
                JavaVersion::Java16 |
                JavaVersion::Java17 => "https://corretto.aws/downloads/latest/amazon-corretto-17-x64-linux-jdk.tar.gz",
                JavaVersion::Java21 => "https://corretto.aws/downloads/latest/amazon-corretto-21-x64-linux-jdk.tar.gz",
                JavaVersion::Java25 => "https://corretto.aws/downloads/latest/amazon-corretto-25-x64-linux-jdk.tar.gz",
                JavaVersion::Java8 => "https://corretto.aws/downloads/latest/amazon-corretto-8-x64-linux-jdk.tar.gz",
            });
        } else if #[cfg(target_arch = "aarch64")] {
            return Some(match self {
                JavaVersion::Java16 |
                JavaVersion::Java17 => "https://corretto.aws/downloads/latest/amazon-corretto-17-aarch64-linux-jdk.tar.gz",
                JavaVersion::Java21 => "https://corretto.aws/downloads/latest/amazon-corretto-21-aarch64-linux-jdk.tar.gz",
                JavaVersion::Java25 => "https://corretto.aws/downloads/latest/amazon-corretto-25-aarch64-linux-jdk.tar.gz",
                JavaVersion::Java8 => "https://corretto.aws/downloads/latest/amazon-corretto-8-aarch64-linux-jdk.tar.gz",
            });
        } else if #[cfg(target_arch = "arm")] {
            return matches!(self, JavaVersion::Java8).then_some(
                "https://github.com/Mrmayman/get-jdk/releases/download/java8-1/jdk-8u231-linux-arm32-vfp-hflt.tar.gz"
            );
        } else if #[cfg(target_arch = "x86")] {
            return matches!(self, JavaVersion::Java8).then_some(
                "https://github.com/hmsjy2017/get-jdk/releases/download/v8u231/jdk-8u231-linux-i586.tar.gz"
            );
        });
        #[allow(unreachable_code)]
        None
    }

    #[cfg(target_os = "macos")]
    fn get_url_macos(self) -> Option<&'static str> {
        cfg_if!(if #[cfg(target_arch = "x86_64")] {
            Some(match self {
                JavaVersion::Java16 |
                JavaVersion::Java17 => "https://corretto.aws/downloads/latest/amazon-corretto-17-x64-macos-jdk.tar.gz",
                JavaVersion::Java21 => "https://corretto.aws/downloads/latest/amazon-corretto-21-x64-macos-jdk.tar.gz",
                JavaVersion::Java25 => "https://corretto.aws/downloads/latest/amazon-corretto-25-x64-macos-jdk.tar.gz",
                JavaVersion::Java8  => "https://corretto.aws/downloads/latest/amazon-corretto-8-x64-macos-jdk.tar.gz",
            })
        } else if #[cfg(target_arch = "aarch64")] {
            Some(match self {
                JavaVersion::Java16 |
                JavaVersion::Java17 => "https://corretto.aws/downloads/latest/amazon-corretto-17-aarch64-macos-jdk.tar.gz",
                JavaVersion::Java21 => "https://corretto.aws/downloads/latest/amazon-corretto-21-aarch64-macos-jdk.tar.gz",
                JavaVersion::Java25 => "https://corretto.aws/downloads/latest/amazon-corretto-25-aarch64-macos-jdk.tar.gz",
                JavaVersion::Java8  => "https://corretto.aws/downloads/latest/amazon-corretto-8-aarch64-macos-jdk.tar.gz",
            })
        });
        #[allow(unreachable_code)]
        None
    }

    #[cfg(target_os = "windows")]
    fn get_url_windows(self) -> Option<&'static str> {
        cfg_if!(if #[cfg(target_arch = "x86_64")] {
            return Some(match self {
                JavaVersion::Java16 |
                JavaVersion::Java17 => "https://corretto.aws/downloads/latest/amazon-corretto-17-x64-windows-jdk.zip",
                JavaVersion::Java21 => "https://corretto.aws/downloads/latest/amazon-corretto-21-x64-windows-jdk.zip",
                JavaVersion::Java25 => "https://corretto.aws/downloads/latest/amazon-corretto-25-x64-windows-jdk.zip",
                JavaVersion::Java8  => "https://corretto.aws/downloads/latest/amazon-corretto-8-x64-windows-jdk.zip",
            });
        } else if #[cfg(target_arch = "x86")] {
            return Some(match self {
                JavaVersion::Java16 |
                JavaVersion::Java17 => "https://corretto.aws/downloads/latest/amazon-corretto-17-x86-windows-jdk.zip",
                JavaVersion::Java21 => "https://corretto.aws/downloads/latest/amazon-corretto-21-x86-windows-jdk.zip",
                JavaVersion::Java25 => "https://corretto.aws/downloads/latest/amazon-corretto-25-x86-windows-jdk.zip",
                JavaVersion::Java8  => "https://corretto.aws/downloads/latest/amazon-corretto-8-x86-windows-jdk.zip",
            });
        });
        #[allow(unreachable_code)]
        None
    }
}
