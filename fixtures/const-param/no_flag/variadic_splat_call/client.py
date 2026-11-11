def fetch(url, timeout, **kwargs):
    return (url, timeout, sorted(kwargs))


def join(sep, *parts):
    return sep.join(parts)


def a(options):
    return fetch("a", 1, **options)


def b():
    return fetch("b", 2)


def c(words):
    return join(",", *words)


def d():
    return join(";")
