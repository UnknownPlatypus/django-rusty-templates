import pytest
from django.template import engines
from django.template.exceptions import TemplateSyntaxError


def test_autoescape_off(assert_render):
    html = "<p>Hello World!</p>"
    template = "{% autoescape off %}{{ html }}{% endautoescape %}"
    assert_render(template=template, context={"html": html}, expected=html)


def test_missing_argument():
    template = "{% autoescape %}{{ html }}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    assert str(exc_info.value) == "'autoescape' tag requires exactly one argument."

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    expected = """\
  × 'autoescape' tag missing an 'on' or 'off' argument.
   ╭────
 1 │ {% autoescape %}{{ html }}
   ·              ▲
   ·              ╰── here
   ╰────
"""
    assert str(exc_info.value) == expected


def test_invalid_argument():
    template = "{% autoescape foo %}{{ html }}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    assert str(exc_info.value) == "'autoescape' argument should be 'on' or 'off'"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    expected = """\
  × 'autoescape' argument should be 'on' or 'off'.
   ╭────
 1 │ {% autoescape foo %}{{ html }}
   ·               ─┬─
   ·                ╰── here
   ╰────
"""
    assert str(exc_info.value) == expected


def test_extra_argument():
    template = "{% autoescape on off %}{{ html }}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    assert str(exc_info.value) == "'autoescape' tag requires exactly one argument."

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    expected = """\
  × 'autoescape' tag requires exactly one argument.
   ╭────
 1 │ {% autoescape on off %}{{ html }}
   ·               ───┬──
   ·                  ╰── here
   ╰────
"""
    assert str(exc_info.value) == expected


def test_missing_endautoescape():
    template = "{% autoescape off %}{{ html }}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    expected = (
        "Unclosed tag on line 1: 'autoescape'. Looking for one of: endautoescape."
    )
    assert str(exc_info.value) == expected

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    expected = """\
  × Unclosed 'autoescape' tag. Looking for one of: endautoescape
   ╭────
 1 │ {% autoescape off %}{{ html }}
   · ──────────┬─────────
   ·           ╰── started here
   ╰────
"""
    assert str(exc_info.value) == expected


def test_wrong_end_tag():
    template = "{% autoescape off %}{{ html }}{% endverbatim %}{% endautoescape %}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    expected = "Invalid block tag on line 1: 'endverbatim', expected 'endautoescape'. Did you forget to register or load this tag?"
    assert str(exc_info.value) == expected

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    expected = """\
  × Unexpected tag endverbatim, expected endautoescape
   ╭────
 1 │ {% autoescape off %}{{ html }}{% endverbatim %}{% endautoescape %}
   · ──────────┬─────────          ────────┬────────
   ·           │                           ╰── unexpected tag
   ·           ╰── start tag
   ╰────
"""
    assert str(exc_info.value) == expected


def test_endautoescape_argument(assert_render):
    html = "<p>Hello World!</p>"
    template = "{% autoescape off %}{{ html }}{% endautoescape extra %}"
    assert_render(template=template, context={"html": html}, expected=html)


def test_nested_autoescape(assert_render):
    html = "<p>Hello World!</p>"
    template = "{{ html }}{% autoescape off %}{{ html }}{% autoescape on %}{{ html }}{% endautoescape %}{% endautoescape %}"
    escaped = "&lt;p&gt;Hello World!&lt;/p&gt;"
    assert_render(
        template=template, context={"html": html}, expected=f"{escaped}{html}{escaped}"
    )


def test_autoescape_text(assert_render):
    template = "{% autoescape off %}<p>Hello World!</p>{% endautoescape %}"
    assert_render(template=template, context={}, expected="<p>Hello World!</p>")


def test_autoescape_comment(assert_render):
    template = "{% autoescape off %}{# comment #}{% endautoescape %}"
    assert_render(template=template, context={}, expected="")


def test_autoescape_url(assert_render):
    template = "{% autoescape off %}{% url 'home' %}{% endautoescape %}"
    assert_render(template=template, context={}, expected="/")


def test_unexpected_end_tag():
    template = "{% endautoescape %}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    expected = "Invalid block tag on line 1: 'endautoescape'. Did you forget to register or load this tag?"
    assert str(exc_info.value) == expected

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    expected = """\
  × Unexpected tag endautoescape
   ╭────
 1 │ {% endautoescape %}
   · ─────────┬─────────
   ·          ╰── unexpected tag
   ╰────
"""
    assert str(exc_info.value) == expected
