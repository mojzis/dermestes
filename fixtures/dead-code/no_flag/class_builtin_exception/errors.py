class _StaleCacheError(Exception):
    pass


class _RetryableError(_StaleCacheError):
    pass


def main():
    print("report")


if __name__ == "__main__":
    main()
