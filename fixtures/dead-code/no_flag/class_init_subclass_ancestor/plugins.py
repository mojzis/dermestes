class Plugin:
    seen = []

    def __init_subclass__(cls):
        Plugin.seen.append(cls)


class _CsvPlugin(Plugin):
    kind = "csv"


def main():
    print(Plugin.seen)


if __name__ == "__main__":
    main()
