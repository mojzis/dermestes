def call_handlers(label, handlers, *args, **kwargs):
    return [(label, handler(*args, **kwargs)) for handler in handlers]


def joined(handlers, worker):
    return call_handlers("join", handlers, worker)


def left(handlers, worker):
    return call_handlers("leave", handlers, worker)
