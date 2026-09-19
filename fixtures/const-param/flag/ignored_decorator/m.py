import functools


@functools.cache
def load(name, strict=True):
    return name if strict else None


def boot(name):
    return load(name)
