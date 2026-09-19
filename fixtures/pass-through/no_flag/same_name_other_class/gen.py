class Generator:
    def _get_instructions(self, x):
        return x.strip()


class Other:
    def _get_instructions(self, x):
        y = x.lower()
        return y

    def build(self, x):
        y = self._get_instructions(x)
        return y
