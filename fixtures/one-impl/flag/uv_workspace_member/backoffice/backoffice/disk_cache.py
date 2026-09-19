from backoffice.cache import CacheBackend


class DiskCache(CacheBackend):
    def get(self, key: str) -> bytes | None:
        return None
