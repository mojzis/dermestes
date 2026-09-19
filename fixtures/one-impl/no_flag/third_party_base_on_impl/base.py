from abc import ABC, abstractmethod


class Model(ABC):
    @abstractmethod
    def save(self): ...
