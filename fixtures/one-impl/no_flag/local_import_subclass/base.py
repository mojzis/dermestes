from abc import ABC, abstractmethod


class Step(ABC):
    @abstractmethod
    def run(self): ...


class FirstStep(Step):
    def run(self):
        return 1
