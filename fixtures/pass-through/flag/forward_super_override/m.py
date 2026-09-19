class Base:
    def save(self, item):
        print(item)
        return item


class Child(Base):
    def save(self, item):
        return super().save(item)

    def flush(self, items):
        for item in items:
            self.save(item)
