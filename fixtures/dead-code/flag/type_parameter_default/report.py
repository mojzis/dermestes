class Box[T = int]:
    pass


def _old_first[T = int, *Ts = *tuple[int, str]](rows: list[T]) -> T:
    return rows[0]


def main():
    print(Box())


if __name__ == "__main__":
    main()
