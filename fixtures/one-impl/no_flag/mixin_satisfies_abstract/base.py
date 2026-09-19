from abc import ABC, abstractmethod


class Source(ABC):
    @abstractmethod
    def fetch(self): ...


class FetchMixin:
    def fetch(self):
        return 1


class HttpSource(FetchMixin, Source):
    pass


class FileSource(Source):
    def fetch(self):
        return 2
