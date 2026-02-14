use serde::{Deserialize, Serialize};
use std::fmt;

/// List of LWJGL 3.x modules that need to be downloaded
pub const LWJGL3_MODULES: &[&str] = &[
    "lwjgl",
    "lwjgl-freetype",
    "lwjgl-glfw",
    "lwjgl-jemalloc",
    "lwjgl-openal",
    "lwjgl-opengl",
    "lwjgl-stb",
    "lwjgl-tinyfd",
];

/// List of LWJGL 2.x modules that need to be downloaded
pub const LWJGL2_MODULES: &[&str] = &["lwjgl", "lwjgl_util"];

/// Represents a list of available LWJGL versions fetched from Maven
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LwjglVersionList {
    /// LWJGL 3.x versions, ordered newest first
    pub lwjgl3: Vec<String>,
    /// LWJGL 2.x versions, ordered newest first
    pub lwjgl2: Vec<String>,
}

/// Wrapper for optional LWJGL version override
/// None means "Default (from game)"
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LwjglVersion(pub Option<String>);

impl LwjglVersion {
    pub fn default_from_game() -> Self {
        Self(None)
    }

    pub fn custom(version: String) -> Self {
        Self(Some(version))
    }

    pub fn is_default(&self) -> bool {
        self.0.is_none()
    }

    pub fn as_str(&self) -> Option<&str> {
        self.0.as_deref()
    }
}

impl fmt::Display for LwjglVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Some(version) => write!(f, "{version}"),
            None => write!(f, "Default (from game)"),
        }
    }
}

impl Default for LwjglVersion {
    fn default() -> Self {
        Self::default_from_game()
    }
}

/// Check if a version string represents LWJGL 3.x
pub fn is_lwjgl3(version: &str) -> bool {
    version.starts_with('3')
}

/// Check if a version string represents LWJGL version before 3.3.2
/// lwjgl-freetype module was first released in 3.3.2
pub fn is_before_332(version: &str) -> bool {
    if !is_lwjgl3(version) {
        // LWJGL 2.x is always before 3.3.2
        return true;
    }

    // Parse version string "3.3.1" -> (3, 3, 1)
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() < 2 {
        return true; // Malformed version, assume it's old
    }
    // Major version (first part)
    if let Ok(major) = parts[0].parse::<u32>() {
        if major < 3 {
            return true;
        }
        if major > 3 {
            return false;
        }
    }

    // Minor version (second part)
    if let Ok(minor) = parts[1].parse::<u32>() {
        if minor < 3 {
            return true;
        }
        if minor > 3 {
            return false;
        }
    }

    // Patch version (third part) - only matters if major=3, minor=3
    if parts.len() >= 3 {
        if let Ok(patch) = parts[2].parse::<u32>() {
            return patch < 2;
        }
    }

    false
}

/// Get the Maven group ID for the given LWJGL version
/// LWJGL 3.x: "org.lwjgl"
/// LWJGL 2.x: "org.lwjgl.lwjgl"
pub fn get_group_id(version: &str) -> &'static str {
    if is_lwjgl3(version) {
        "org.lwjgl"
    } else {
        "org.lwjgl.lwjgl"
    }
}

/// Get the native classifier for the current platform
/// Returns the Maven classifier used for native libraries (e.g., "natives-linux-arm64")
pub fn get_native_classifier() -> Option<&'static str> {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    return Some("natives-linux");

    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    return Some("natives-linux-arm64");

    #[cfg(all(target_os = "linux", target_arch = "arm"))]
    return Some("natives-linux-arm32");

    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    return Some("natives-windows");

    #[cfg(all(target_os = "windows", target_arch = "aarch64"))]
    return Some("natives-windows-arm64");

    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    return Some("natives-macos");

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    return Some("natives-macos-arm64");

    #[cfg(all(target_os = "freebsd", target_arch = "x86_64"))]
    return Some("natives-freebsd");

    #[cfg(not(any(
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "aarch64"),
        all(target_os = "linux", target_arch = "arm"),
        all(target_os = "windows", target_arch = "x86_64"),
        all(target_os = "windows", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "freebsd", target_arch = "x86_64"),
    )))]
    None
}

/// Get the native classifier for LWJGL 2.x
/// LWJGL 2.x uses slightly different naming (e.g., "natives-osx" instead of "natives-macos")
pub fn get_native_classifier_lwjgl2() -> Option<&'static str> {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    return Some("natives-linux");

    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    return Some("natives-linux");

    #[cfg(all(target_os = "linux", target_arch = "arm"))]
    return Some("natives-linux");

    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    return Some("natives-windows");

    #[cfg(all(target_os = "windows", target_arch = "aarch64"))]
    return Some("natives-windows");

    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    return Some("natives-osx");

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    return Some("natives-osx");

    #[cfg(all(target_os = "freebsd", target_arch = "x86_64"))]
    return Some("natives-freebsd");

    #[cfg(not(any(
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "aarch64"),
        all(target_os = "linux", target_arch = "arm"),
        all(target_os = "windows", target_arch = "x86_64"),
        all(target_os = "windows", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "freebsd", target_arch = "x86_64"),
    )))]
    None
}

/// Build Maven Central URL for fetching version metadata
pub fn build_lwjgl_maven_metadata_url(version: &str) -> String {
    let group_id = get_group_id(version);
    let group_path = group_id.replace('.', "/");

    if is_lwjgl3(version) {
        // LWJGL 3.x: https://repo1.maven.org/maven2/org/lwjgl/lwjgl/maven-metadata.xml
        format!("https://repo1.maven.org/maven2/{group_path}/lwjgl/maven-metadata.xml")
    } else {
        // LWJGL 2.x: https://repo1.maven.org/maven2/org/lwjgl/lwjgl/lwjgl/maven-metadata.xml
        format!("https://repo1.maven.org/maven2/{group_path}/lwjgl/maven-metadata.xml")
    }
}

/// Build Maven Central URL for downloading a specific LWJGL artifact
/// Examples:
/// - LWJGL 3.x: https://repo1.maven.org/maven2/org/lwjgl/lwjgl-opengl/3.3.1/lwjgl-opengl-3.3.1.jar
/// - LWJGL 3.x natives: https://repo1.maven.org/maven2/org/lwjgl/lwjgl-opengl/3.3.1/lwjgl-opengl-3.3.1-natives-linux-arm64.jar
/// - LWJGL 2.x: https://repo1.maven.org/maven2/org/lwjgl/lwjgl/lwjgl/2.9.4/lwjgl-2.9.4.jar
pub fn build_lwjgl_maven_url(version: &str, module: &str, classifier: Option<&str>) -> String {
    let group_id = get_group_id(version);
    let group_path = group_id.replace('.', "/");

    let artifact_id = if is_lwjgl3(version) {
        module.to_string()
    } else {
        // LWJGL 2.x on Maven Central:
        // - main jars: lwjgl, lwjgl_util
        // - natives jars: lwjgl-platform (with classifier natives-*)
        if classifier.is_some_and(|c| c.starts_with("natives-")) {
            "lwjgl-platform".to_string()
        } else if module == "lwjgl_util" {
            "lwjgl_util".to_string()
        } else {
            "lwjgl".to_string()
        }
    };

    let filename = if let Some(classifier) = classifier {
        format!("{artifact_id}-{version}-{classifier}.jar")
    } else {
        format!("{artifact_id}-{version}.jar")
    };

    format!("https://repo1.maven.org/maven2/{group_path}/{artifact_id}/{version}/{filename}")
}

/// Build local library path for a LWJGL artifact
/// Examples:
/// - libraries/org/lwjgl/lwjgl/3.3.1/lwjgl-3.3.1.jar
/// - libraries/org/lwjgl/lwjgl-opengl/3.3.1/lwjgl-opengl-3.3.1-natives-linux-arm64.jar
pub fn build_lwjgl_library_path(version: &str, module: &str, classifier: Option<&str>) -> String {
    let group_id = get_group_id(version);
    let group_path = group_id.replace('.', "/");

    let artifact_id = if is_lwjgl3(version) {
        module.to_string()
    } else if classifier.is_some_and(|c| c.starts_with("natives-")) {
        "lwjgl-platform".to_string()
    } else if module == "lwjgl_util" {
        "lwjgl_util".to_string()
    } else {
        "lwjgl".to_string()
    };

    let filename = if let Some(classifier) = classifier {
        format!("{artifact_id}-{version}-{classifier}.jar")
    } else {
        format!("{artifact_id}-{version}.jar")
    };

    format!("{group_path}/{artifact_id}/{version}/{filename}")
}

/// Fetch available LWJGL versions from Maven Central
pub async fn fetch_lwjgl_versions() -> Result<LwjglVersionList, String> {
    // Fetch both LWJGL 3.x and 2.x versions
    let lwjgl3_url = "https://repo1.maven.org/maven2/org/lwjgl/lwjgl/maven-metadata.xml";
    let lwjgl2_url = "https://repo1.maven.org/maven2/org/lwjgl/lwjgl/lwjgl/maven-metadata.xml";

    let mut all_versions = Vec::new();

    // Fetch LWJGL 3.x versions
    match reqwest::get(lwjgl3_url).await {
        Ok(response) => {
            if let Ok(text) = response.text().await {
                let mut versions = parse_maven_metadata(&text);
                all_versions.append(&mut versions);
            }
        }
        Err(e) => {
            return Err(format!("Failed to fetch LWJGL 3.x versions: {e}"));
        }
    }

    // Fetch LWJGL 2.x versions
    match reqwest::get(lwjgl2_url).await {
        Ok(response) => {
            if let Ok(text) = response.text().await {
                let mut versions = parse_maven_metadata(&text);
                all_versions.append(&mut versions);
            }
        }
        Err(e) => {
            return Err(format!("Failed to fetch LWJGL 2.x versions: {e}"));
        }
    }

    if all_versions.is_empty() {
        return Err("No LWJGL versions found".to_string());
    }

    // Separate LWJGL 3.x and 2.x versions
    let mut lwjgl3 = Vec::new();
    let mut lwjgl2 = Vec::new();

    for version in all_versions {
        if is_lwjgl3(&version) {
            lwjgl3.push(version);
        } else {
            lwjgl2.push(version);
        }
    }

    // Sort each list: newest first
    lwjgl3.sort_by(|a, b| b.cmp(a));
    lwjgl2.sort_by(|a, b| b.cmp(a));

    Ok(LwjglVersionList { lwjgl3, lwjgl2 })
}

/// Parse Maven metadata XML to extract version numbers
fn parse_maven_metadata(xml: &str) -> Vec<String> {
    let mut versions = Vec::new();
    let mut remaining = xml;

    while let Some(start_idx) = remaining.find("<version>") {
        let after_tag = &remaining[start_idx + 9..];
        if let Some(end_idx) = after_tag.find("</version>") {
            let version = after_tag[..end_idx].trim().to_owned();
            versions.push(version);
            remaining = &after_tag[end_idx + 10..];
        } else {
            break;
        }
    }

    versions
}
