import threading


class Stage(threading.Thread):
    def validate_schema(self, rows):
        raise NotImplementedError


class ParseStage(Stage):
    def validate_schema(self, rows):
        return all(rows)


def main():
    ParseStage().start()


if __name__ == "__main__":
    main()
