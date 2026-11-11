def render(doc, mode="html"):
    if doc is None:
        return ""
    elif mode == "text":
        return doc
    return "<p>" + doc + "</p>"


def a():
    return render("a")


def b(doc):
    return render(doc)
