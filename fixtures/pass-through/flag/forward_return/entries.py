def resolve_entry(store, key):
    return store[key]


def resolve_relation(store, key):
    """Resolve a relation by key."""
    return resolve_entry(store, key)


def show(store, key):
    found = resolve_relation(store, key)
    return found
