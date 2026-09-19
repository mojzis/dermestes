class Printer:
    def _emit(self, line, debug=False):
        if debug:
            print("debug", line)
        print(line)

    def show(self, lines):
        for line in lines:
            self._emit(line)
