from typing import Any

from rich import print as rprint


def _show(obj: Any) -> None:
    rprint(obj)


def a(x):
    _show(x)


def b(x):
    _show(x)
