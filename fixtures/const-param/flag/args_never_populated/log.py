def log(level, *args):
    return (level, len(args))


def warn():
    return log("warn")


def info():
    return log("info")
