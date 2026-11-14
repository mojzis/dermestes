try:
    from scripts.sec import units
except ImportError:
    units = None


def built(report):
    return units(report, "Built units")
