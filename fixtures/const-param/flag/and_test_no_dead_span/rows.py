def load(path, strict=False):
    rows = open(path).read().split()
    if strict and not rows:
        raise ValueError(path)
    while (strict := not strict):
        rows.pop()
    return rows


def a():
    return load("a.txt")


def b(name):
    return load(name)
