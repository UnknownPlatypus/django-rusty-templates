from django.utils.translation import override

from .utils import render


def test_translate_default():
    expected = "Welcome\n"

    assert render("translation.txt", {}, using="django") == expected


def test_translate_missing():
    expected = "Welcome\n"

    with override("fr"):  # Deliberately missing translation
        assert render("translation.txt", {}, using="django") == expected


def test_translate_valid():
    expected = "Willkommen\n"
    with override("de"):
        assert render("translation.txt", {}, using="django") == expected
