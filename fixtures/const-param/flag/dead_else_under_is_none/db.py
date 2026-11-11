def connect(url, pool=None):
    if pool is None:
        conn = open(url)
    else:
        conn = pool.get(url)
        conn.reset()
    return conn


def a():
    return connect("a")


def b(url):
    return connect(url)
