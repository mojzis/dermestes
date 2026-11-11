import asyncio


async def push_rows(rows, sink):
    await asyncio.sleep(0)
    sink.extend(rows)
    return sink, rows


async def nightly(rows, sink):
    await push_rows(rows, sink)
