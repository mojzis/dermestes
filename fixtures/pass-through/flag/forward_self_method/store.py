class Store:
    def __init__(self, root):
        self.root = root

    def _read(self, key, mode):
        path = self.root + key
        return open(path, mode).read()

    def _load(self, key):
        return self._read(key, "r")

    def refresh(self, key):
        data = self._load(key)
        self._read(key, "rb")
        return data
