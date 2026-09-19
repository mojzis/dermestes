from typing import Any


def _show(obj: Any) -> None:
    obj.show()


def run(x):
    _show(x)
    return x
