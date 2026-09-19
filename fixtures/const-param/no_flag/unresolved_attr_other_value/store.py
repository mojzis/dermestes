class Transcript:
    def done(self, cost_usd=None):
        return cost_usd

    def finish(self):
        return self.done()


def close(t):
    return t.done(cost_usd=0.5)
