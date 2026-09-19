class Point:
    def __init__(self, x=0):
        self.x = x


def origin(registry):
    registry.add(Point)
    return Point()
