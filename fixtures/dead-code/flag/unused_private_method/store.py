class Store:
    def load(self):
        with open("data.txt") as handle:
            return handle.read()

    def _legacy_read(self):
        return []


if __name__ == "__main__":
    print(Store().load())
