def handler(event, code, **kwargs):
    return (event, code, kwargs)


def install(bus):
    bus.subscribe(handler)


def a():
    return handler("a", 1)


def b():
    return handler("b", 2)
