from abc import ABC, abstractmethod


class Codec(ABC):
    @abstractmethod
    def encode(self, data): ...


class Json(Codec):
    def encode(self, data):
        return str(data)


class Yaml(Codec):
    def encode(self, data):
        return str(data)
