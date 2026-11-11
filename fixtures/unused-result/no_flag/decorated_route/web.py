from fastapi import FastAPI

app = FastAPI()


@app.post("/sync")
def sync(rows):
    return rows


def warm(rows):
    sync(rows)
