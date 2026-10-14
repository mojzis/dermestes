SESSION_KIND = "chat"


def create_stats(kind, scope, data):
    return (kind, scope, data)


def _create_session_stats(data):
    return create_stats(SESSION_KIND, "session", data)


def a(data):
    return _create_session_stats(data)


def b(data):
    return _create_session_stats(data)


def other(data):
    return create_stats("batch", "run", data)
