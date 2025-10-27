import argparse
import os
import shutil
import sys

from . import launch, procs, create, run_suite
from .launch import IS_X11, IS_XWAYLAND, IS_WINDOWS

IS_COMPATIBLE: bool = IS_X11 or IS_XWAYLAND or IS_WINDOWS

if not IS_COMPATIBLE:
    print("""Unsupported platform!
This test suite only currently supports:
- Windows
- X11 (Linux, etc)
- XWayland under Wayland (Linux, etc)

macOS and Wayland-without-XWayland systems
aren't supported yet.

For more info see tests/README.md
""")
    sys.exit(1)


def rmdir(directory_path: str) -> None:
    if os.path.exists(directory_path) and os.path.isdir(directory_path):
        shutil.rmtree(directory_path)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--timeout", type=int, default=60, help="Timeout in seconds (default: 60)"
    )
    parser.add_argument(
        "--existing",
        action="store_true",
        help="Use existing instances from previous test, instead of redownloading",
    )
    parser.add_argument(
        "--lwjgl2",
        action="store_true",
        help="Only test LWJGL2 (1.12-) versions, not LWJGL3 (1.13+)",
    )
    # parser.add_argument("--instance", required=True, help="Instance ID to test")
    args = parser.parse_args()
    print("(building launcher...)")
    procs.run(["cargo", "build"])

    procs.prepare_ql_bin()

    # Customize as needed
    versions = create.VERSIONS_L2
    if not args.lwjgl2:
        versions += create.VERSIONS_L3

    if not args.existing:
        rmdir("tests/QuantumLauncher/instances")
        rmdir("tests/QuantumLauncher/assets")
        create.create(versions)

    launch.TIMEOUT = args.timeout
    run_suite.test(versions)


if __name__ == "__main__":
    main()
