import sys


class _JsonPlugin:
    def load(self, text):
        return text


def main():
    plugin = getattr(sys.modules[__name__], "_JsonPlugin")()
    print(plugin.load("x"))


if __name__ == "__main__":
    main()
