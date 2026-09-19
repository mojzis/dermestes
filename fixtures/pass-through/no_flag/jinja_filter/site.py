import jinja2


def slugify(text):
    return text.lower().replace(" ", "-")


def slug(text):
    return slugify(text)


env = jinja2.Environment()
env.filters["slug"] = slug


def title(text):
    out = slug(text)
    return out
