class Stage:
    def validate_schema(self, rows):
        raise NotImplementedError


class ParseStage(Stage):
    def validate_schema(self, rows):
        return all(rows)


def main(stage, rows):
    print(stage.validate_schema(rows))


if __name__ == "__main__":
    main(ParseStage(), [])
