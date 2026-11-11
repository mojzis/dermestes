class Base:
    def run(self, name, *args, **kwargs):
        return (name, args, kwargs)

    def a(self):
        return self.run("a")


class Sub(Base):
    def run(self, name, *args, **kwargs):
        return name

    def b(self):
        return self.run("b")
