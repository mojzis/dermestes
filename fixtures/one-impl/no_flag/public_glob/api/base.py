from abc import ABC, abstractmethod


class Hook(ABC):
    @abstractmethod
    def fire(self): ...


class OnlyHook(Hook):
    def fire(self):
        return 1
