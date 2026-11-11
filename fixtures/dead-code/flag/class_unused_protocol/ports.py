from typing import Protocol, runtime_checkable


@runtime_checkable
class SupportsFlush(Protocol):
    def flush_buffers(self) -> None: ...


def main():
    print("report")


if __name__ == "__main__":
    main()
