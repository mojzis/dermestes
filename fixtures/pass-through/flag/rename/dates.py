from datetime import date


def _parse(v):
    return date.fromisoformat(v)


def start(raw):
    return _parse(raw)


def end(raw):
    return _parse(raw)
