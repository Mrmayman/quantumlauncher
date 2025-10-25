use ql_core::Loader;

pub struct Version(pub &'static str, pub &'static [Loader]);

const fn ver(name: &'static str) -> Version {
    Version(name, &[])
}

const MODERN: [Loader; 2] = [Loader::Forge, Loader::Quilt];

pub const VERSIONS_LWJGL2: &[Version] = &[
    // last version of classic, should represent most early versions
    ver("c0.30-c-1900"),
    ver("a1.1.2_01"), // one of the most popular alpha versions
    ver("b1.7.3"),    // most popular beta version
    // last based on old launcher system
    Version("1.5.2", &[Loader::Quilt]),
    // after migration to new launcher system
    Version("1.7.10", &MODERN),
    // one of the most popular release versions
    Version("1.8.9", &MODERN),
    // last version to use lwjgl2
    Version("1.12.2", &MODERN),
];

pub const VERSIONS_LWJGL3: &[Version] = &[
    ver("inf-20100415-lwjgl3"), // test of lwjgl3 backport
    Version("1.14.4", &MODERN), // after migration to lwjgl3, engine rewrites
    Version("1.16.5", &MODERN), // last version to use Java 8, OpenGL 2.x
    // after migration to Java 17, OpenGL 3.x, engine rewrites
    Version("1.18.2", &MODERN),
    // last launchwrapper version
    Version("1.21.5", &[Loader::Forge, Loader::Quilt, Loader::Neoforge]),
    // latest
    Version("1.21.10", &[Loader::Forge, Loader::Quilt, Loader::Neoforge]),
];
