from abc import ABC, abstractmethod

from pluggy import HookSpec


class Plugin(HookSpec, ABC):
    @abstractmethod
    def run(self): ...


class OnlyPlugin(Plugin):
    def run(self):
        return 1
