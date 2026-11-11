from typing import Any, Callable, ParamSpec

_P = ParamSpec("_P")


def _guarded(label: str, f: Callable[_P, Any], *args: _P.args, **kw: _P.kwargs) -> Any:
    try:
        return f(*args, **kw)
    except ValueError:
        return label


def a(x):
    return _guarded("a", int, x)


def b(x):
    return _guarded("b", float, x)
