TIMEOUT = 5


def wait_ready(host, timeout):
    return host, timeout


def boot(host):
    return wait_ready(host, TIMEOUT)
