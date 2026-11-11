def push_rows(rows, sink):
    sink.extend(rows)
    return rows


def install(scheduler):
    scheduler.every(60, push_rows)


def nightly(rows, sink):
    push_rows(rows, sink)
