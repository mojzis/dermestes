import logging


class TimeOnlyFormatter(logging.Formatter):
    def formatTime(self, record, datefmt=None):
        return ""
