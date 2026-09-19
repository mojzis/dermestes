from abc import ABC, abstractmethod


class Shape(ABC):
    @abstractmethod
    def area(self): ...

    @abstractmethod
    def name(self): ...


class Named(Shape):
    def name(self):
        return type(self).__name__


class Square(Named):
    def area(self):
        return 4
