from base import Exporter


class CsvExporter(Exporter):
    def export(self, rows):
        return ",".join(rows)
