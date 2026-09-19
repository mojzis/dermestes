def extract_content(page, debug=False):
    if debug:
        print(page)
    return page


def run(pages):
    return [extract_content(p) for p in pages]
