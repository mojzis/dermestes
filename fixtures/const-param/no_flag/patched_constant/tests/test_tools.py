import tools


def test_fast(monkeypatch):
    monkeypatch.setattr("tools.REFRESH_TIMEOUT", 0.2)
    assert tools.refresh() == 0.2
