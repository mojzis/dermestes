ID_LENGTH = 16


def make_id(length, prefix):
    return prefix + "x" * length


def generate_id(prefix):
    return make_id(ID_LENGTH, prefix)


def a():
    return generate_id("a")


def b():
    return make_id(8, "b")


def c():
    return generate_id("c")
