# Run with scripts/ on sys.path.
from jira import api


def summary(key):
    return api.view(key, fields="summary")
