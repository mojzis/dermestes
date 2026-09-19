from fmt import _fmt


def test_fmt():
    assert _fmt(1) == "1.00"
    assert _fmt(2) == "2.00"
