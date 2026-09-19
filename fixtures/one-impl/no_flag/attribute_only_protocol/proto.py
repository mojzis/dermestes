from typing import Protocol


class HasName(Protocol):
    name: str


class User:
    name: str = "x"

    def greet(self):
        return self.name
