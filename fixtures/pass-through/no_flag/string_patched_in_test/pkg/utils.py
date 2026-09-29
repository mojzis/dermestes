import pathlib


def _load_raw_text(file_path: pathlib.Path) -> str:
    return file_path.read_text()


def load_file(file_path: pathlib.Path) -> str:
    if file_path.suffix == ".sql":
        return _load_raw_text(file_path)
    return ""
