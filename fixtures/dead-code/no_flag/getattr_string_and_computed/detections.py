def is_email(value):
    return "@" in value


def _is_iso_date(value):
    return value.count("-") == 2
