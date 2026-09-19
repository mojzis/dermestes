class Stage:
    def run(self, item):
        raise NotImplementedError


class ParseStage(Stage):
    def run(self, item):
        return item.strip()
