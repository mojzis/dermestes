from client import fetch, log


def test_fetch():
    assert fetch("a", 1, retries=3)
    assert log("debug", "x", "y")
