from . import procs
from .types import Loader, Version

MODERN = {Loader.FORGE, Loader.FABRIC, Loader.QUILT}

# LWJGL 2 tests
VERSIONS_L2: list[Version] = [
    # last version of classic, should represent most early versions
    Version("c0.30-c-1900"),
    Version("a1.1.2_01"),  # one of the most popular alpha versions
    Version("b1.7.3"),  # most popular beta version
    # last based on old launcher system
    Version("1.5.2", {Loader.FABRIC, Loader.QUILT}),
    # after migration to new launcher system
    Version("1.7.10", MODERN),
    # one of the most popular release versions
    Version("1.8.9", MODERN),
    # last version to use lwjgl2
    Version("1.12.2", MODERN),
]

# LWJGL 3 tests
VERSIONS_L3: list[Version] = [
    Version("inf-20100415-lwjgl3"),  # test of lwjgl3 backport
    Version("1.14.4", MODERN),  # after migration to lwjgl3, engine rewrites
    Version("1.16.5", MODERN),  # last version to use Java 8, OpenGL 2.x
    # after migration to Java 17, OpenGL 3.x, engine rewrites
    Version("1.18.2", MODERN),
    Version("1.21.5", MODERN | {Loader.NEOFORGE}),  # last launchwrapper version
    Version("1.21.10", MODERN | {Loader.NEOFORGE}),  # latest
]


def create(versions: list[Version] | None = None) -> None:
    versions = versions or (VERSIONS_L2 + VERSIONS_L3)
    procs.run_parallel(
        [
            [procs.QL_BIN, "create", version.name, version.name, "-s"]
            for version in versions
        ]
    )
    print("\nFinished creating all instances! Proceeding to running them!\n")
