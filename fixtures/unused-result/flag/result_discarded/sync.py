def push_rows(rows, sink):
    if not rows:
        return 0
    for row in rows:
        sink.append(row)
    count = len(sink)
    return count


def nightly(rows, sink):
    push_rows(rows, sink)


def hourly(rows, sink):
    push_rows(rows[:10], sink)
