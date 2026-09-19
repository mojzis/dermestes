import config
from config import TIMEOUT


def fetch(url, timeout):
    return (url, timeout)


def fetch_all(urls):
    first = fetch(urls[0], TIMEOUT)
    return [first] + [fetch(u, timeout=config.TIMEOUT) for u in urls]
