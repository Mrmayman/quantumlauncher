use cfg_if::cfg_if;

cfg_if!(if #[cfg(any(feature = "simulate_linux_arm64"))] {
    pub const OS_NAME: &str = "linux";
    pub const OS_NAMES: &[&str] = &["linux"];
} else if #[cfg(any(target_os = "macos", feature = "simulate_macos_arm64"))] {
    pub const OS_NAME: &str = "osx";
    pub const OS_NAMES: &[&str] = &["macos", "osx"];
} else {
    pub const OS_NAME: &str = std::env::consts::OS;
    pub const OS_NAMES: &[&str] = &[OS_NAME];
});

cfg_if!(if #[cfg(any(
    target_arch = "aarch64",
    feature = "simulate_linux_arm64",
    feature = "simulate_macos_arm64"
))] {
    pub const ARCH: &str = "arm64";
} else if #[cfg(target_arch = "arm")] {
    pub const ARCH: &str = "arm32";
} else if #[cfg(target_arch = "x86")] {
    pub const ARCH: &str = "x86";
});

pub const DEFAULT_RAM_MB_FOR_INSTANCE: usize = 2048;
