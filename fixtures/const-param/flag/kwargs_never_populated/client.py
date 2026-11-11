def fetch(url, timeout, **kwargs):
    return (url, timeout, sorted(kwargs))


def a():
    return fetch("a", 1)


def b():
    return fetch("b", timeout=2)
