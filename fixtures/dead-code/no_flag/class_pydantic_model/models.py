from pydantic import BaseModel


class _Settings(BaseModel):
    debug: bool = False


def main():
    print("report")


if __name__ == "__main__":
    main()
