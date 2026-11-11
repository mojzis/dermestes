def push_rows(rows, sink):
    sink.extend(rows)
    return rows


def nightly(rows, sink):
    push_rows(rows, sink)
