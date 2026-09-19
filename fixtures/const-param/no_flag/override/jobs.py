import logging


class Base:
    def run(self, fast=False):
        return fast


class Sub(Base):
    def run(self, fast=False):
        return not fast


class Handler(logging.Handler):
    def emit(self, record, extra=None):
        return record

    def flush_all(self, records):
        for r in records:
            self.emit(r)


def go():
    return Sub().run() and Base().run()
