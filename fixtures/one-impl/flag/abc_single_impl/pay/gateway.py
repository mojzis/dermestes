from abc import ABC, abstractmethod


class PaymentGateway(ABC):
    @abstractmethod
    def charge(self, amount): ...

    @abstractmethod
    def refund(self, charge_id): ...

    @abstractmethod
    def status(self, charge_id): ...
