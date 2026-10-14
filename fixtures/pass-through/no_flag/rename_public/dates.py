from datetime import date


def parse(v):
    return date.fromisoformat(v)


def start(raw):
    return parse(raw)


def end(raw):
    return parse(raw)
