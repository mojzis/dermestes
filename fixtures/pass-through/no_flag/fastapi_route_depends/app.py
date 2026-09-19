import os

from fastapi import Depends, FastAPI

app = FastAPI()
ENV = os.environ["ENV"]


def load_settings(env):
    return {"env": env}


def get_settings(env):
    return load_settings(env)


@app.get("/")
def index(settings=Depends(get_settings)):
    return settings


def warm():
    settings = get_settings(ENV)
    return settings
