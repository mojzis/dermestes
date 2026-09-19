from app import StdoutSink


def test_write():
    StdoutSink().write("x")
