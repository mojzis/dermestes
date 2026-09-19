from abc import ABC, abstractmethod

__all__ = ["Handler"]


class Handler(ABC):
    @abstractmethod
    def handle(self, event): ...


class LogHandler(Handler):
    def handle(self, event):
        return event
