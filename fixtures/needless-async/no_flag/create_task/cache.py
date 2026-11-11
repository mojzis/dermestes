import asyncio


async def load_index(path):
    return open(path).read().split()


async def warm(path):
    task = asyncio.create_task(load_index(path))
    return await task


async def reload(path):
    return await load_index(path)
