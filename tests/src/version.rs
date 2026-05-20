use ql_core::Loader;

pub struct Version(pub &'static str, pub &'static [Loader]);

const fn ver(name: &'static str) -> Version {
    Version(name, &[])
}

const FORGE_QUILT: [Loader; 1] = [Loader::Forge];

pub const VERSIONS_LWJGL2: &[Version] = &[
    // last version of classic, should represent most early versions
    ver("c0.30-c-1900"),
    ver("a1.1.2_01"), // one of the most popular alpha versions
    ver("b1.7.3"),    // most popular beta version
    // last based on old launcher system
    Version("1.5.2", &[]),
    // after migration to new launcher system
    Version("1.7.10", &FORGE_QUILT),
    // one of the most popular release versions
    Version("1.8.9", &FORGE_QUILT),
    // last version to use lwjgl2
    Version("1.12.2", &FORGE_QUILT),
];

pub const VERSIONS_LWJGL3: &[Version] = &[
    ver("inf-20100415-lwjgl3"),      // test of lwjgl3 backport
    Version("1.14.4", &FORGE_QUILT), // after migration to lwjgl3, engine rewrites
    Version("1.16.5", &FORGE_QUILT), // last version to use Java 8, OpenGL 2.x
    // after migration to Java 17, OpenGL 3.x, engine rewrites
    Version("1.18.2", &FORGE_QUILT),
    // has some weird bugs
    Version("1.21.5", &[Loader::Forge, Loader::Quilt, Loader::NeoForge]),
    // last launchwrapper version
    Version("1.21.10", &[Loader::Forge, Loader::Quilt, Loader::NeoForge]),
    // TODO: Wait for a version to come out after 1.21.11, then add it here
    // (BetterJSONs is, unfortunately, up-to-date so can't test without it)
];
