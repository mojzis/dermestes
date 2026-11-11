from typing import Protocol


class Source(Protocol):
    async def read(self, key):
        return None

    async def read_many(self, keys):
        return [await self.read(key) for key in keys]
