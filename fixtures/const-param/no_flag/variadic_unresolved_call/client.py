class Client:
    def fetch(self, url, **kwargs):
        return (url, sorted(kwargs))

    def home(self):
        return self.fetch("/")


def page(client, url):
    return client.fetch(url)
