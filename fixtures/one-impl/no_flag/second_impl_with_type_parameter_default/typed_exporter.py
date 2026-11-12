from base import Exporter


class TypedExporter[T = str](Exporter):
    def export(self, rows):
        return list(rows)
