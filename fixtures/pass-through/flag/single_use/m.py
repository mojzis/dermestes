def _label(name):
    return name.strip().title()


def render(names):
    out = []
    for n in names:
        out.append(_label(n))
    return out
