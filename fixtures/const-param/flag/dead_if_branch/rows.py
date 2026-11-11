def load(path, strict=False):
    rows = open(path).read().split()
    if strict:
        for row in rows:
            if not row:
                raise ValueError(path)
    return rows


def a():
    return load("a.txt")


def b(name):
    return load(name)
