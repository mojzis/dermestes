def get(key):
    print(key)
    return key


def find(jobs, key):
    return jobs.get(key)


def main(j, k):
    found = find(j, k)
    get(k)
    return found
