class _Point:
    x = 0
    y = 0


def norm(point: "_Point") -> int:
    return point.x + point.y


def main():
    print(norm)


if __name__ == "__main__":
    main()
