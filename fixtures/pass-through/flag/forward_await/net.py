async def fetch(url, retries):
    return url, retries


async def fetch_page(url):
    await fetch(url, 3)


async def main(urls):
    for url in urls:
        await fetch_page(url)
    await fetch(urls[0], 1)
