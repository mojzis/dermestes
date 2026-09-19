class Config:
    @classmethod
    def load(cls, path, strict=True):
        return cls() if strict else path


def boot(path):
    return Config.load(path)
