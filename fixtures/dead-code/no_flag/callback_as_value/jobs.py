import threading
from functools import partial

from fastapi import Depends


def _worker():
    print("work")


def _scaled(x, factor):
    return x * factor


def _get_db():
    return object()


DB = Depends(_get_db)


if __name__ == "__main__":
    threading.Thread(target=_worker).start()
    double = partial(_scaled, factor=2)
    print(double(3))
