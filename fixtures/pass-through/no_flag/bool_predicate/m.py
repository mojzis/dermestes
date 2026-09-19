def _is_ready(state) -> bool:
    return state.get("ready") is True


def main(s):
    if _is_ready(s):
        return 1
    return 0
