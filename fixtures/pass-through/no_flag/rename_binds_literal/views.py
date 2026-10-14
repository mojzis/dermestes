def _user_id(request):
    return request.session.get("user_id")


def a(request):
    return _user_id(request)


def b(request):
    return _user_id(request)
