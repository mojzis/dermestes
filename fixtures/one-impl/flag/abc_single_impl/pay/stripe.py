from pay.gateway import PaymentGateway


class StripeGateway(PaymentGateway):
    def charge(self, amount):
        return amount

    def refund(self, charge_id):
        return charge_id

    def status(self, charge_id):
        return "ok"
