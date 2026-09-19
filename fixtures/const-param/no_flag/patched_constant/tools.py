REFRESH_TIMEOUT = 30.0


def wait(finish_timeout):
    return finish_timeout


def refresh():
    return wait(finish_timeout=REFRESH_TIMEOUT)
