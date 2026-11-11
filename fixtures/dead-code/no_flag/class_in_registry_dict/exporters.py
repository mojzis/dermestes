class _CsvExporter:
    def export(self, rows):
        return ",".join(rows)


def main():
    print(EXPORTERS["csv"]().export([]))


EXPORTERS = {"csv": _CsvExporter}

if __name__ == "__main__":
    main()
