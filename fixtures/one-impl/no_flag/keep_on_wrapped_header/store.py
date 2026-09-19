from abc import ABC, abstractmethod


class PanelStoreWithLongName(
    ABC
):  # dermestes: keep second backend (S3) lands next sprint, see issue 41
    @abstractmethod
    def save_panel(self, name: str, data: bytes) -> None: ...


class DiskPanelStore(PanelStoreWithLongName):
    def save_panel(self, name: str, data: bytes) -> None:
        pass
