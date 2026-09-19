def render(text, width=80):
    return text[:width]


def a():
    render("a")
    render("b", 80)
