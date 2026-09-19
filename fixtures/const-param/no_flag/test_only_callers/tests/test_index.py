from index import append_index


def test_append():
    assert append_index("a") == "todos/a"
