from abc import ABC, abstractmethod


class Exporter(ABC):
    @abstractmethod
    def export(self): ...


class CsvExporter(Exporter):
    def export(self):
        return ""
