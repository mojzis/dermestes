from celery import shared_task


@shared_task
def sync(account, mode, **kwargs):
    return (account, mode, kwargs)


def a():
    return sync("a", 1)


def b():
    return sync("b", 2)
