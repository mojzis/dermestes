def extract_content(page, debug=False):
    return page if debug else page


def run(pages):
    return [extract_content(p) for p in pages]
