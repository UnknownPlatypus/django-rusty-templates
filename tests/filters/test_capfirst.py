"""
Adapted from
https://github.com/django/django/blob/5.1/tests/template_tests/filter_tests/test_capfirst.py
"""

import pytest
from django.utils.safestring import mark_safe


@pytest.mark.xfail(reason="autoescape not ready yet")
def test_capfirst01(self):
    """
    @setup(
        {
            "capfirst01": (
                "{% autoescape off %}{{ a|capfirst }} {{ b|capfirst }}"
                "{% endautoescape %}"
            )
        }
    )
    """
    output = self.engine.render_to_string(
        "capfirst01", {"a": "fred>", "b": mark_safe("fred&gt;")}
    )
    self.assertEqual(output, "Fred> Fred&gt;")


def test_capfirst02(assert_render):
    template = "{{ a|capfirst }} {{ b|capfirst }}"
    expected = "Fred&gt; Fred&gt;"
    context = {"a": "fred>", "b": mark_safe("fred&gt;")}

    assert_render(template, context, expected)


def test_capfirst(assert_render):
    template = "{{ a|capfirst }}"
    context = {"a": "hello world"}

    assert_render(template, context, "Hello world")


def test_capfirst_for_list(assert_render):
    template = "{{ a|capfirst }}"
    context = {"a": ["hello"]}

    assert_render(template, context, "[&#x27;hello&#x27;]")
