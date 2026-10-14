def _fetch_cost_rows(conn):
    return conn.execute("select 1").fetchall()


def main():
    print("report")


if __name__ == "__main__":
    main()
