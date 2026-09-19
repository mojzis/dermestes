import core.sinks


class StdoutSink(core.sinks.Sink):
    def write(self, data):
        print(data)
