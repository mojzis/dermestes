class _Registry(type):
    seen = []

    def __new__(mcs, name, bases, namespace):
        cls = super().__new__(mcs, name, bases, namespace)
        mcs.seen.append(cls)
        return cls


class _Plugin(metaclass=_Registry):
    pass


def main():
    print("report")


if __name__ == "__main__":
    main()
