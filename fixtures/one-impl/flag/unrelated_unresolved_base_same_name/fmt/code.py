from abc import ABC, abstractmethod


class Formatter(ABC):
    @abstractmethod
    def format_code(self, source: str) -> str: ...


class BlackFormatter(Formatter):
    def format_code(self, source: str) -> str:
        return source
