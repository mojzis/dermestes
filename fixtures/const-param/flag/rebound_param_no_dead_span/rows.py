def load(path, strict=False):
    if path.endswith(".strict"):
        strict = True
    if strict:
        return open(path).read().split()
    return []


def a():
    return load("a.txt")


def b(name):
    return load(name)
