import marimo
from fastapi import FastAPI

api = FastAPI()
app = marimo.App()


@api.get("/")
def home():
    return {"ok": True}


@app.cell
def _():
    return
