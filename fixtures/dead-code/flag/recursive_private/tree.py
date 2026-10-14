def _walk(node):
    for child in node.children:
        _walk(child)
