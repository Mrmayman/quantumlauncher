from . import procs
from .types import Version

# LWJGL 2 tests
VERSIONS_L2: list[Version] = [
    "c0.30-c-1900",  # last version of classic, should represent most early versions

    "a1.1.2_01",  # one of the most popular alpha versions
    "b1.7.3",  # most popular beta version
    "1.8.9",  # one of the most popular release versions

    "1.5.2",  # last based on old launcher system
    "1.7.10",  # after migration to new launcher system
    "1.12.2"  # last version to use lwjgl2
]

# LWJGL 3 tests
VERSIONS_L3: list[Version] = [
    "inf-20100415-lwjgl3",  # test of lwjgl3 backport

    "1.14.4",  # after migration to lwjgl3, engine rewrites
    "1.16.5",  # last version to use Java 8, OpenGL 2.x
    "1.18.2",  # after migration to Java 17, OpenGL 3.x, engine rewrites
    "1.21.5",  # last LaunchWrapper version
    "1.21.8",  # latest
]


def create(versions: list[Version] | None = None) -> None:
    versions = versions or (VERSIONS_L2 + VERSIONS_L3)
    procs.run_parallel([
        [procs.QL_BIN, "create", version, version, "-s"]
        for version in versions
    ])
    print("\nFinished creating all instances! Proceeding to running them!\n")
