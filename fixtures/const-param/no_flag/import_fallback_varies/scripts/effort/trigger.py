try:
    from scripts.sec import units
except ImportError:
    from sec import units


def built(report):
    return units(report, "Built units")
