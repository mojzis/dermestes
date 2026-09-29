def _refresh_status(session, force):
    if force:
        session.reload()
    return session.status


def refresh_status(session, force):
    return _refresh_status(session, force)


def run(session):
    return refresh_status(session, True), refresh_status(session, False), _refresh_status(session, False)
