def compose_strips(strips, gap=0, margin=0, border=0, strict=True):
    if strict and not strips:
        raise ValueError("no strips")
    return [gap + margin + border for _ in strips]


def page(strips):
    return compose_strips(strips)


def cover(strips):
    return compose_strips(strips, strict=True)
