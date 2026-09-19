__all__ = ["pad"]


def pad(x, width=80):
    return str(x).ljust(width)


def show(x):
    return pad(x)
