from fastapi import FastAPI

app = FastAPI()


@app.get("/")
def index(page=1):
    return page


def home():
    return index()
