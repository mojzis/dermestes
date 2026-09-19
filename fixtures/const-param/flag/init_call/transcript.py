class Transcript:
    def __init__(self, path, cost_usd=0.0):
        self.path = path
        self.cost_usd = cost_usd


def open_transcript(path):
    return Transcript(path)
