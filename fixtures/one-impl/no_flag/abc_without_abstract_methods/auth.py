# From feast (permissions/auth/auth_manager.py): an ABC with no abstract
# methods is instantiable, and feast does instantiate it directly.
from abc import ABC


class AuthManager(ABC):
    def __init__(self, parser):
        self.parser = parser


class AllowAll(AuthManager):
    def __init__(self):
        super().__init__(parser=None)


def default():
    return AuthManager(parser="jwt")
