def _fmt(value):
    return f"{value:.2f}"


def report(values):
    lines = [_fmt(v) for v in values]
    return lines
