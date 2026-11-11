class Store:
    def flush(self):
        return 3

    def close(self):
        self.flush()


def drain(store):
    return store.flush() + 1
