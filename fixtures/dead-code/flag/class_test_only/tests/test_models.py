from models import _CostRow


def test_cost_row():
    assert _CostRow("a", 1.0).usd == 1.0
