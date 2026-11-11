import asyncio


async def load_index(path):
    async with asyncio.timeout(5):
        return open(path).read().split()


async def scan(stream, path):
    async for chunk in stream:
        print(chunk, path)


async def warm(stream, path):
    await scan(stream, path)
    return await load_index(path)
