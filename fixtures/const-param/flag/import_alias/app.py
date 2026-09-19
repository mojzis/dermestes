import tools.text as tt
from tools.text import slugify as sl


def title(s):
    return tt.slugify(s) + sl(s, sep="-")
