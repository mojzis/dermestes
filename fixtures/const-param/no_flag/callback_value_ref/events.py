HANDLERS = []


def handler(evt, retries=3):
    return (evt, retries)


def setup(bus):
    bus.subscribe(handler)
    handler("boot")
