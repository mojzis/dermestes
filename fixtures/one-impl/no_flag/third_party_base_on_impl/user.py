from pydantic import BaseModel

from base import Model


class User(Model, BaseModel):
    def save(self):
        return None
