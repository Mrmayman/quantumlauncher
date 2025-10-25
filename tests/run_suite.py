import sys
from tests import launch
from . import procs
from .types import LOADER_ALL, Version
import subprocess


def uninstall(instance):
    subprocess.run([procs.QL_BIN, "loaders", "uninstall", instance])


def get_all_instances() -> list[str]:
    result = subprocess.run([procs.QL_BIN, "-l"], text=True, capture_output=True)
    return result.stdout.splitlines()


def uninstall_all():
    instances = get_all_instances()
    for instance in instances:
        uninstall(instance)


def test(versions: list[Version]):
    for ver in versions:
        subprocess.run([procs.QL_BIN, "loader", "uninstall", ver.name])
        if not launch.test(ver):
            sys.exit(1)
        for loader in LOADER_ALL:
            if loader in ver.loaders:
                print(f"- Testing loader {loader.value}")
                subprocess.run(
                    [procs.QL_BIN, "loader", "install", ver.name, loader.value]
                )
                if not launch.test(ver):
                    sys.exit(1)
                subprocess.run([procs.QL_BIN, "loader", "uninstall", ver.name])
