from abc import ABC, abstractmethod


class CacheBackend(ABC):
    @abstractmethod
    def get(self, key: str) -> bytes | None: ...
