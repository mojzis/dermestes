try:
    from fast import units
except ImportError:
    from slow import units


def built(report):
    return units(report, "Built units")
