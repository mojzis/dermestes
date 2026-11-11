def fetch(url, timeout, **kwargs):
    return (url, timeout, sorted(kwargs))


def log(level, *args):
    return (level, len(args))


def a():
    return fetch("a", 1), log("a")


def b():
    return fetch("b", 2), log("b")
