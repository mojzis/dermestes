from extract import extract_content


def test_extract():
    assert extract_content("x", debug=False) == "x"
