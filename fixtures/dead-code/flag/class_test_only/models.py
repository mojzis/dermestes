from dataclasses import dataclass


@dataclass(frozen=True)
class _CostRow:
    name: str
    usd: float


def main():
    print("report")


if __name__ == "__main__":
    main()
