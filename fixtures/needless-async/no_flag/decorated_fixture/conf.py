import pytest_asyncio


@pytest_asyncio.fixture
async def settings():
    return {"debug": True}


async def boot():
    return await settings()
