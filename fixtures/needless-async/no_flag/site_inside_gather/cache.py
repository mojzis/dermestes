import asyncio


async def load_index(path):
    return open(path).read().split()


async def warm(paths):
    first = await load_index(paths[0])
    rest = await asyncio.gather(load_index(paths[1]), load_index(paths[2]))
    return first, rest
