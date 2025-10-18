"""
Adapted from
https://github.com/django/django/blob/5.1/tests/template_tests/filter_tests/test_capfirst.py
"""

import pytest
from django.utils.safestring import mark_safe


@pytest.mark.parametrize(
    "template,context,expected",
    [
        pytest.param(
            "{% autoescape off %}{{ a|capfirst }} {{ b|capfirst }}{% endautoescape %}",
            {"a": "fred>", "b": mark_safe("fred&gt;")},
            "Fred> Fred&gt;",
            id="capfirst01_autoescape_off",
        ),
        pytest.param(
            "{{ a|capfirst }} {{ b|capfirst }}",
            {"a": "fred>", "b": mark_safe("fred&gt;")},
            "Fred&gt; Fred&gt;",
            id="capfirst02_autoescape_on",
        ),
        pytest.param(
            "{{ a|capfirst }}",
            {"a": "hello world"},
            "Hello world",
            id="capfirst_basic",
        ),
        pytest.param(
            "{{ a|capfirst }}",
            {"a": ["hello"]},
            "[&#x27;hello&#x27;]",
            id="capfirst_for_list",
        ),
    ],
)
def test_capfirst(assert_render, template, context, expected):
    assert_render(template, context, expected)
