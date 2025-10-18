"""
Test cases adapted from
https://github.com/django/django/blob/main/tests/template_tests/filter_tests/test_addslashes.py
"""

import pytest
from django.utils.safestring import mark_safe


@pytest.mark.parametrize(
    "template,context,expected",
    [
        pytest.param(
            "{% autoescape off %}{{ a|addslashes }} {{ b|addslashes }}{% endautoescape %}",
            {"a": "<a>'", "b": mark_safe("<a>'")},
            r"<a>\' <a>\'",
            id="addslashes_autoescape_off",
        ),
        pytest.param(
            "{{ a|addslashes }} {{ b|addslashes }}",
            {"a": "<a>'", "b": mark_safe("<a>'")},
            r"&lt;a&gt;\&#x27; <a>\'",
            id="addslashes_autoescape_on",
        ),
        pytest.param(
            "{{ a|addslashes }} and {{ b|addslashes }}",
            {"a": mark_safe('"double quotes"'), "b": mark_safe("'single quotes'")},
            r"\"double quotes\" and \'single quotes\'",
            id="addslashes_quotes",
        ),
    ],
)
def test_addslashes(assert_render, template, context, expected):
    assert_render(template, context, expected)


def test_backslashes(assert_render):
    template = "{{ a|addslashes }}"
    context = {"a": r"\ : backslashes, too"}
    expected = r"\\ : backslashes, too"

    assert_render(template, context, expected)


def test_non_string_input(assert_render):
    template = "{{ a|addslashes }}"
    context = {"a": 123}

    assert_render(template, context, "123")
