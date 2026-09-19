from abc import ABC, abstractmethod


class Unused(ABC):
    @abstractmethod
    def go(self): ...
