from abc import ABC, abstractmethod

from registry import AutoRegister


class Task(ABC):
    @abstractmethod
    def run(self): ...


class OnlyTask(Task, metaclass=AutoRegister):
    def run(self):
        return 1
