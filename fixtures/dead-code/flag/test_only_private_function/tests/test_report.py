from report import _fetch_cost_rows


def test_fetch_cost_rows(conn):
    assert _fetch_cost_rows(conn) == []
