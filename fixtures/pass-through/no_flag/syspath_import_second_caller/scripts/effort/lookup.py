# Run with scripts/ on sys.path.
from jira import api


def summary(project, number):
    return api._key(project, number)
