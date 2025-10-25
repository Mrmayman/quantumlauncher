from dataclasses import dataclass
from enum import Enum

PID = int


class Loader(Enum):
    FORGE = "forge"
    NEOFORGE = "neoforge"
    FABRIC = "fabric"
    QUILT = "quilt"


LOADER_ALL: list[Loader] = [
    Loader.FORGE,
    Loader.NEOFORGE,
    Loader.FABRIC,
    Loader.QUILT,
]


@dataclass
class Version:
    name: str
    loaders: set[Loader]

    def __init__(self, name: str, loaders: set[Loader] | Loader = None):
        self.name = name
        if loaders is None:
            self.loaders = set()
        elif isinstance(loaders, Loader):
            self.loaders = {loaders}
        else:
            self.loaders = loaders

    def __str__(self):
        return self.name
