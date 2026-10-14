import sys

import detections


class Plugin:
    def _hook(self):
        return 1


def run(plugin):
    return getattr(plugin, "_hook")()


def detect(kind, value):
    return getattr(detections, "is_" + kind)(value)


if __name__ == "__main__":
    print(run(Plugin()), detect(sys.argv[1], sys.argv[2]))
