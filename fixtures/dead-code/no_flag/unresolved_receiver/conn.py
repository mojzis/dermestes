class Connection:
    def close(self):
        print("closed")


def shutdown(conn):
    conn.close()


if __name__ == "__main__":
    shutdown(Connection())
