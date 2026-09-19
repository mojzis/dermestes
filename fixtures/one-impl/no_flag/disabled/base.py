from abc import ABC, abstractmethod


class Job(ABC):
    @abstractmethod
    def run(self): ...


class OnlyJob(Job):
    def run(self):
        return 1
