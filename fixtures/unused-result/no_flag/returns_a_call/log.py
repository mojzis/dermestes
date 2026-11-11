import logging

logger = logging.getLogger(__name__)


def debug(msg, step):
    return logger.debug("%s %s", msg, step)


def wrap_all(handlers, wrap):
    return [wrap(handler) for handler in handlers]


def index_or_noop(store, key, found):
    if found:
        return {"result": "noop"}
    return store.index(key)


def boot(handlers, wrap, store, key, found):
    debug("starting", 1)
    wrap_all(handlers, wrap)
    index_or_noop(store, key, found)
    debug("started", 2)
