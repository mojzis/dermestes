def emit(msg, level):
    print(level, msg)


def log(msg):
    emit(msg, "info")


def run(a, b):
    log(a)
    log(b)
    emit(b, "debug")
