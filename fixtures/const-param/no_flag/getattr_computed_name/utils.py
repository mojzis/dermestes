import detections


def detect(name, v):
    return getattr(detections, "is_" + name)(v, False)


def check(v):
    return detections.is_hex(v)
