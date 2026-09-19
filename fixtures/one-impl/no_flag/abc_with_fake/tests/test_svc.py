from svc import Mailer


class FakeMailer(Mailer):
    def send(self, to):
        return None
