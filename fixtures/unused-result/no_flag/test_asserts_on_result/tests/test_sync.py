from sync import push_rows


def test_push():
    assert push_rows([1], []) == [1]
