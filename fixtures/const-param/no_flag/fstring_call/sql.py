def rollup(alias=""):
    return f"{alias}.n"


def a():
    return rollup()


def b():
    return f"SELECT {rollup('cr')} FROM t"
