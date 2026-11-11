class _LegacyRow:
    def __init__(self, raw):
        self.raw = raw

    def parse_legacy(self):
        return self.raw.split(",")


def main():
    print("report")


if __name__ == "__main__":
    main()
