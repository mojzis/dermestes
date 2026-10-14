from textwrap import dedent


def _clean(text):
    return dedent(text)


def a(text):
    return _clean(text)


def b(text, dedent):
    return dedent(_clean(text))
