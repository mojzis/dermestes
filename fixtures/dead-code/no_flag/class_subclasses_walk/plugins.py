class Plugin:
    kind = ""


class _CsvPlugin(Plugin):
    kind = "csv"


def main():
    print([cls.kind for cls in Plugin.__subclasses__()])


if __name__ == "__main__":
    main()
