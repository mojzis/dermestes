from typing import Protocol


class Callback(Protocol):
    def __call__(self, value: int) -> None: ...


class Printer:
    def __call__(self, value: int) -> None:
        print(value)
