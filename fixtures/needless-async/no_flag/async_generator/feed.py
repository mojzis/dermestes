async def ticks(limit):
    for tick in range(limit):
        yield tick


async def first(limit):
    async for tick in ticks(limit):
        return tick


async def stream(limit):
    return await ticks(limit).__anext__()
