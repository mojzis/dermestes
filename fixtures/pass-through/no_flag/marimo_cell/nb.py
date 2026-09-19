import marimo

app = marimo.App()


def helper(x):
    return x


@app.cell
def show(x):
    return helper(x)


def main(v):
    out = show(v)
    return out
