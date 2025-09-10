// TUI licenses helper: centralizes license list and embedded fallbacks

/// Index of the combined "About & Licenses" entry in the left settings menu
pub const fn menu_index() -> usize {
    4
}

/// Returns list of license display names and file path candidates
pub fn entries() -> &'static [(&'static str, &'static [&'static str])] {
    &[
        (
            "QuantumLauncher (GPLv3)",
            &["LICENSE", "../LICENSE", "../../LICENSE"],
        ),
        (
            "Forge Installer",
            &[
                "assets/licenses/APACHE_2.txt",
                "../assets/licenses/APACHE_2.txt",
                "../../assets/licenses/APACHE_2.txt",
            ],
        ),
        (
            "Fonts (Inter/Jetbrains Mono)",
            &[
                "assets/licenses/OFL.txt",
                "../assets/licenses/OFL.txt",
                "../../assets/licenses/OFL.txt",
            ],
        ),
        (
            "Password Asterisks Font",
            &[
                "assets/licenses/CC_BY_SA_3_0.txt",
                "../assets/licenses/CC_BY_SA_3_0.txt",
                "../../assets/licenses/CC_BY_SA_3_0.txt",
            ],
        ),
        (
            "LWJGL",
            &[
                "assets/licenses/LWJGL.txt",
                "../assets/licenses/LWJGL.txt",
                "../../assets/licenses/LWJGL.txt",
            ],
        ),
    ]
}

/// Compile-time embedded license text fallbacks (mirrors the Iced UI behavior)
/// Index corresponds to `entries()` ordering
pub fn fallback_content(index: usize) -> Option<&'static str> {
    match index {
        // QuantumLauncher GPLv3
        0 => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../LICENSE"
        ))),
        // Forge Installer (Apache 2.0)
        1 => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../assets/licenses/APACHE_2.txt"
        ))),
        // Fonts (OFL)
        2 => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../assets/licenses/OFL.txt"
        ))),
        // Password Asterisks Font (CC BY-SA 3.0)
        3 => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../assets/licenses/CC_BY_SA_3_0.txt"
        ))),
        // LWJGL
        4 => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../assets/licenses/LWJGL.txt"
        ))),
        _ => None,
    }
}
