import hashlib


def body_revision(authored: str) -> str:
    return hashlib.sha256(authored.encode()).hexdigest()


def _body_revision(authored: str) -> str:
    return body_revision(authored)


def update_entry_partial(authored: str, body_revision: str | None) -> bool:
    return _body_revision(authored) == body_revision

