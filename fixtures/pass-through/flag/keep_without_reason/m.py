def search(query, source):
    return query, source


def search_mcp(query):  # dermestes: keep
    return search(query, "mcp")


def main(q):
    found = search_mcp(q)
    search(q, "web")
    return found
