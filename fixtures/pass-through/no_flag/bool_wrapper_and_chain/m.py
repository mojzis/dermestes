def check(x):
    print(x)
    return x


def is_ok(x):
    return bool(check(x))


def real_of(x):
    return check(x).real


def main(v):
    a = is_ok(v)
    b = real_of(v)
    return a, b
