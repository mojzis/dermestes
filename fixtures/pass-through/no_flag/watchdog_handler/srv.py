from watchdog.events import FileSystemEventHandler


class Handler(FileSystemEventHandler):
    def _rebuild(self, path):
        print(path)
        return path

    def on_modified(self, event):
        self._rebuild(event)

    def poke(self, event):
        self.on_modified(event)
        return event
