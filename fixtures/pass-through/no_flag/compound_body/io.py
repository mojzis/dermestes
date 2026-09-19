def _write(path, text):
    with open(path, "w") as f:
        f.write(text)


def _echo_all(lines):
    for line in lines:
        print(line)


def save(path, text):
    _write(path, text)
    _echo_all([text])
