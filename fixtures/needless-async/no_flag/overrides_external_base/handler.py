from aiohttp import web


class Health(web.View):
    async def get(self):
        return web.Response(text="ok")

    async def post(self):
        return await self.get()
