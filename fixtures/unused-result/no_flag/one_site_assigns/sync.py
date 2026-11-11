def push_rows(rows, sink):
    sink.extend(rows)
    return rows


def nightly(rows, sink):
    push_rows(rows, sink)


def hourly(rows, sink):
    count = push_rows(rows[:10], sink)
    print(count)
