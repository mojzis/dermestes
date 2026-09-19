def query(conn, sql):
    return conn, sql


def a(conn):
    return query(conn, "select 1")


def b(conn):
    return query(conn, "select 2")
