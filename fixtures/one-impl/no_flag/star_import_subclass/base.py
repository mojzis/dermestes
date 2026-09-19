from abc import ABC, abstractmethod


class Rule(ABC):
    @abstractmethod
    def check(self): ...


class FirstRule(Rule):
    def check(self):
        return True
