def emit(level, msg):
    print(level, msg)


def log(level, msg):
    emit(level, msg)


def run():
    log(1, "a")
