def emit(rows):
    for row in rows:
        yield row
    return rows


def drain(rows):
    emit(rows)
