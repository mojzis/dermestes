from typing import Protocol


class Greeter(Protocol):
    def greet(self, name: str) -> str: ...

    def farewell(self, name: str) -> str: ...


def welcome(greeter: Greeter) -> str:
    return greeter.greet("you")
