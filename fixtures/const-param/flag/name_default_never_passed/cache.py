from pathlib import Path

CACHE_DIR = Path("cache")


def load(key, cache_dir=CACHE_DIR):
    return cache_dir / key


def get(key):
    return load(key)
