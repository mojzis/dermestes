class Point:
    def __init__(self, x=0):
        self.x = x

    @classmethod
    def at(cls, x):
        return cls(x=x)


def origin():
    return Point()
