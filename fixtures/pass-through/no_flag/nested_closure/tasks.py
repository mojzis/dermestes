import asyncio
import threading


def work(x):
    print(x)
    return x


async def work_async(x):
    print(x)
    return x


def start(x):
    def _run():
        return work(x)

    threading.Thread(target=_run).start()

    async def _go():
        await work_async(x)

    asyncio.create_task(_go())
