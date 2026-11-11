import sys


def main(argv=None):
    print(argv or sys.argv)
    return 0


def rerun():
    main(["again"])
