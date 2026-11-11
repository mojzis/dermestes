from sync import push_rows


def test_push():
    sink = []
    push_rows([1], sink)
    assert sink == [1]
