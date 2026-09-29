import pathlib
from unittest import mock

import pytest

from pkg import utils


@pytest.mark.parametrize(("file_name", "fn"), [("file.sql", "_load_raw_text")])
def test_dispatch(file_name, fn):
    with mock.patch(f"pkg.utils.{fn}") as mock_fn:
        mock_fn.return_value = "x"
        assert utils.load_file(pathlib.Path(file_name)) == "x"
