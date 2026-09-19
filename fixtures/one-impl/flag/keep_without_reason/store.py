import abc


class Store(abc.ABC):  # dermestes: keep
    @abc.abstractmethod
    def get(self, key): ...


class DictStore(Store):
    def get(self, key):
        return key
