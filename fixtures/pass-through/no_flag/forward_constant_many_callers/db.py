DB_PATH = "app.db"


def get_connection(path, read_only):
    return (path, read_only)


def _get_read_connection(read_only):
    return get_connection(DB_PATH, read_only)


def a():
    return _get_read_connection(True)


def b():
    return _get_read_connection(True)


def c():
    return _get_read_connection(False)


def write():
    return get_connection("other.db", False)
