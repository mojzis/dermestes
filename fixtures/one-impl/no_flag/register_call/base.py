from abc import ABC, abstractmethod


class Sized(ABC):
    @abstractmethod
    def size(self): ...


class Box(Sized):
    def size(self):
        return 1


Sized.register(tuple)
