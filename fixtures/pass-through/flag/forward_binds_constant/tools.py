TOOL_NAME = "search"


def extract_tool_input(resp, name):
    return resp.get(name)


def _extract_tool_input(resp):
    return extract_tool_input(resp, TOOL_NAME)


def first(resp):
    return _extract_tool_input(resp)


def second(resp):
    return _extract_tool_input(resp) or {}


def other(resp):
    return extract_tool_input(resp, "fetch")
