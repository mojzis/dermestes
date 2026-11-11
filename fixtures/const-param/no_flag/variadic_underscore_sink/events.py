def on_event(name, *_, **_kwargs):
    return name


def a():
    return on_event("a")


def b():
    return on_event("b")
