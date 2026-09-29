class Writer:
    def __init__(self, sink):
        self._sink = sink

    def _write(self, sink, data, flush):
        sink.write(data)
        if flush:
            sink.flush()

    def _emit(self, data):
        self._write(self._sink, data, flush=True)

    def send(self, data):
        self._emit(data)
        self._write(self._sink, data, flush=False)
