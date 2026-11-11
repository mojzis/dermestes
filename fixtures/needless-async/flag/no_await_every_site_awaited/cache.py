async def load_index(path):
    with open(path) as handle:
        return handle.read().split()


async def warm():
    rows = await load_index("a.idx")
    return len(rows)


async def reload(path):
    return await load_index(path)
