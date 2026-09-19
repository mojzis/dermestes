from abc import ABCMeta, abstractmethod


class Loader(metaclass=ABCMeta):
    @abstractmethod
    def load(self): ...
