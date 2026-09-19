from typing import Protocol, runtime_checkable


@runtime_checkable  # dermestes: keep plugin point, see ADR 7
class SessionExporter(Protocol):
    def export_sessions(self, rows: list[dict]) -> bytes: ...


class CsvSessionExporter:
    def export_sessions(self, rows: list[dict]) -> bytes:
        return b""
