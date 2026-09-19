from abc import ABC, abstractmethod


class Mailer(ABC):
    @abstractmethod
    def send(self, to): ...


class SmtpMailer(Mailer):
    def send(self, to):
        return to
