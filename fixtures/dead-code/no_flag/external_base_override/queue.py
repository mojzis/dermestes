import asyncio


class LifoQueue(asyncio.Queue):
    def _init(self, maxsize):
        self._queue = []
