def box(text, width, border=True):
    pad = 2 if border else 0
    return text.center(width + pad)


def title():
    return box("title", 40)


def footer():
    return box("footer", 20)
