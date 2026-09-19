def make():
    from base import Step

    class LocalStep(Step):
        def run(self):
            return 2

    return LocalStep()
