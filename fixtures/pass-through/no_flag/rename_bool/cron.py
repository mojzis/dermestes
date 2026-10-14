from croniter import croniter


def _valid(s) -> bool:
    return croniter.is_valid(s)


def a(s):
    return _valid(s)


def b(s):
    return _valid(s)
