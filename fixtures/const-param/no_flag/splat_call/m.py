def area(w, h=1):
    return w * h


def many(args):
    return [area(*a) for a in args] + [area(2)]
