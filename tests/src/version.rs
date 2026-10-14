use ql_core::Loader;

pub struct Version(pub &'static str, pub &'static [Loader]);

const FABRIC_FAMILY: [Loader; 2] = [Loader::Fabric, Loader::Quilt];
const FORGE_FAB_QLT: [Loader; 3] = [Loader::Forge, Loader::Fabric, Loader::Quilt];
const MODERN: [Loader; 4] = [
    Loader::Fabric,
    Loader::Forge,
    Loader::Quilt,
    Loader::NeoForge,
];

pub const VERSIONS_LWJGL2: &[Version] = &[
    Version("c0.30-c-1900", &FABRIC_FAMILY), // last classic, should represent most early versions
    Version("a1.1.2_01", &FABRIC_FAMILY),    // one of the most popular alpha versions
    Version("b1.7.3", &FABRIC_FAMILY),       // most popular beta version (babric gets tested here)
    Version("1.5.2", &FABRIC_FAMILY),        // last based on old launcher system (legacy tested)
    Version("1.7.10", &FORGE_FAB_QLT),       // after migration to new launcher system
    Version("1.8.9", &FORGE_FAB_QLT),        // one of the most popular release versions
    Version("1.12.2", &FORGE_FAB_QLT),       // last version to use lwjgl2
];

const NOT_FORGE: [Loader; 3] = [Loader::Fabric, Loader::Quilt, Loader::NeoForge];
pub const VERSIONS_LWJGL3: &[Version] = &[
    Version("inf-20100415-lwjgl3", &FABRIC_FAMILY), // test of lwjgl3 backport
    Version("1.14.4", &FORGE_FAB_QLT),              // after migration to lwjgl3, engine rewrites
    Version("1.16.5", &FORGE_FAB_QLT),              // last version to use Java 8, OpenGL 2.x
    Version("1.18.2", &FORGE_FAB_QLT), // after migration to Java 17, OpenGL 3.x, engine rewrites
    Version("1.21.5", &MODERN),        // has some weird bugs
    // naming stuff:
    Version("1.21.1", &MODERN),
    Version("1.21.11", &MODERN),
    Version("26.1-snap1", &NOT_FORGE),
    Version("26.1-snap11", &NOT_FORGE),
    Version("26.1", &MODERN),
    Version("26.4-snapshot-4", &FABRIC_FAMILY),
];
