import requests


def fetch(url, timeout, **kwargs):
    return requests.get(url, timeout=timeout, **kwargs)


def a():
    return fetch("a", 1)


def b():
    return fetch("b", 2)
