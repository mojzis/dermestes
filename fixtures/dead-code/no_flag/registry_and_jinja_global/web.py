import jinja2


def _csv(rows):
    return ",".join(rows)


def url_for_file(path):
    return "/files/" + path


EXPORTERS = {"csv": _csv}
env = jinja2.Environment()
env.globals["file_url"] = url_for_file
