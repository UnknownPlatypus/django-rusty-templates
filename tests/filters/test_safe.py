import pytest
from django.template import engines
from django.template.exceptions import TemplateSyntaxError


def test_safe(assert_render):
    template = "{{ html|safe }}"
    html = "<p>Hello World!</p>"
    assert_render(template=template, context={"html": html}, expected=html)


def test_safe_with_argument():
    template = "{{ html|safe:invalid }}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    assert str(exc_info.value) == "safe requires 1 arguments, 2 provided"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    expected = """\
  × safe filter does not take an argument
   ╭────
 1 │ {{ html|safe:invalid }}
   ·              ───┬───
   ·                 ╰── unexpected argument
   ╰────
"""
    assert str(exc_info.value) == expected


def test_safe_missing_value(assert_render):
    template = "{{ html|safe }}"
    assert_render(template=template, context={}, expected="")


def test_safe_already_safe(assert_render):
    template = "{{ html|safe|safe }}"
    html = "<p>Hello World!</p>"
    assert_render(template=template, context={"html": html}, expected=html)


def test_safe_integer(assert_render):
    template = "{{ num|default:100|safe }}"
    assert_render(template=template, context={}, expected="100")


def test_safe_float(assert_render):
    template = "{{ num|default:1.6|safe }}"
    assert_render(template=template, context={}, expected="1.6")


def test_safe_escaped(assert_render):
    template = "{{ html|escape|safe }}"
    html = "<p>Hello World!</p>"
    escaped = "&lt;p&gt;Hello World!&lt;/p&gt;"
    assert_render(template=template, context={"html": html}, expected=escaped)


def test_safe_bool(assert_render):
    template = "{% for x in 'xy' %}{{ forloop.first|safe }}{% endfor %}"
    assert_render(template=template, context={}, expected="TrueFalse")


def test_safe_autoescape_off(assert_render):
    template = "{% autoescape off %}{{ html|safe }}{% endautoescape %}"
    html = "<p>Hello World!</p>"
    assert_render(template=template, context={"html": html}, expected=html)


def test_safe_autoescape_off_lower(assert_render):
    template = "{% autoescape off %}{{ html|lower|safe }}{% endautoescape %}"
    html = "<p>Hello World!</p>"
    assert_render(template=template, context={"html": html}, expected=html.lower())
