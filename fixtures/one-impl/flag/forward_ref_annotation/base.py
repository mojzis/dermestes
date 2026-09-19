from abc import ABC, abstractmethod
from typing import TypeVar


class Backend(ABC):
    """A Backend runs jobs."""

    @abstractmethod
    def run(self): ...


class LocalBackend(Backend):
    def run(self):
        return 1


B = TypeVar("B", bound="Backend")


def make() -> "Backend":
    return LocalBackend()


def default(backend: "Backend | None" = None) -> "Backend":
    return backend or make()


LABEL = "Backend"
