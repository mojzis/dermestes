def insert_history(conn, duration_s=None):
    return (conn, duration_s)


def save(conn):
    insert_history(conn)
