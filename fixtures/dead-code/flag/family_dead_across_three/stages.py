class Stage:
    def run(self, rows):
        return rows

    def validate_schema(self, rows):
        raise NotImplementedError


class ParseStage(Stage):
    def validate_schema(self, rows):
        return all(rows)


class LoadStage(Stage):
    def validate_schema(self, rows):
        return super().validate_schema(rows)


def main(rows):
    for stage in (ParseStage(), LoadStage(), Stage()):
        rows = stage.run(rows)
    print(rows)


if __name__ == "__main__":
    main([])
