from abc import ABC, abstractmethod


class Backend(ABC):
    @abstractmethod
    def run(self): ...


class LocalBackend(Backend):
    def run(self):
        return 1
