def render(text, border=True):
    return text if border else ""


def page(a, b):
    return render(a, border=True) + render(b)
