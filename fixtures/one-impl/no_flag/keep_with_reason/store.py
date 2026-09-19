import abc


class Store(abc.ABC):  # dermestes: keep plugin point for downstream packages
    @abc.abstractmethod
    def get(self, key): ...


class DictStore(Store):
    def get(self, key):
        return key
