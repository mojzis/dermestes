# Run with scripts/ on sys.path.
from jira.api import view


def summary(key):
    return view(key, fields="summary")
