class Query:
    def __init__(self):
        self.parts = []

    def where(self, clause):
        self.parts.append(clause)
        return self

    def defaults(self, tenant):
        self.where("deleted = 0")
        self.where(tenant)
