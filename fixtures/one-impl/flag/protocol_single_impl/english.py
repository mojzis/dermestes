class English:
    def greet(self, name: str) -> str:
        return f"hello {name}"

    def farewell(self, name: str) -> str:
        return f"bye {name}"


class Unrelated:
    def greet(self, name: str) -> str:
        return name
