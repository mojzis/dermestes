from abc import ABC, abstractmethod


class Repo(ABC):
    @abstractmethod
    def get(self, key): ...


class SqlRepo(Repo):
    @abstractmethod
    def connect(self): ...

    def get(self, key):
        return key
