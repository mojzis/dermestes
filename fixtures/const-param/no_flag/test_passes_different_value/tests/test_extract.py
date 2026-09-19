from extract import extract_content


def test_debug():
    assert extract_content("x", debug=True) == "x"
