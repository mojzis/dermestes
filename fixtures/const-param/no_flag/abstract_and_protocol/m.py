from abc import ABC, abstractmethod
from typing import Protocol


class Runner(ABC):
    @abstractmethod
    def run(self, n=1): ...

    def go(self):
        return self.run()


class Client(Protocol):
    def fetch(self, url, timeout=5): ...

    def fetch_all(self, urls):
        return [self.fetch(u) for u in urls]
