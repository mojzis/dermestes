class Table:
    def __init__(self, rows):
        self.rows = rows

    def _repr_html_(self):
        return "<table></table>"


if __name__ == "__main__":
    print(Table([]))
