class Store:
    def save(self, row):
        return 0

    def save_all(self, rows):
        for row in rows:
            self.save(row)


class DiskStore(Store):
    def save(self, row):
        return row
