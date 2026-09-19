class State:
    def next_uuid(self, line, sub=0):
        return f"{line}:{sub}"

    def read(self, line):
        return f"{self.next_uuid(line)}:read"
