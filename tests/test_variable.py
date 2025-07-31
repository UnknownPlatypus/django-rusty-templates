import pytest
from django.template import engines
from django.template.exceptions import TemplateSyntaxError


def test_render_variable():
    template = "{{ foo }}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    assert django_template.render({"foo": 3}) == "3"
    assert rust_template.render({"foo": 3}) == "3"


def test_render_int():
    template = "{{ 1 }}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    assert django_template.render({"1": 2}) == "1"
    assert rust_template.render({"1": 2}) == "1"


def test_render_float():
    template = "{{ 1.2 }}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    assert django_template.render({"1.2": 2}) == "1.2"
    assert rust_template.render({"1.2": 2}) == "1.2"


def test_render_negative_int():
    template = "{{ -1 }}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    assert django_template.render({"-1": 2}) == "-1"
    assert rust_template.render({"-1": 2}) == "-1"


def test_render_negative_float():
    template = "{{ -1.2 }}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    assert django_template.render({"-1.2": 2}) == "-1.2"
    assert rust_template.render({"-1.2": 2}) == "-1.2"


def test_render_attribute_int():
    template = "{{ foo.1 }}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    assert django_template.render({"foo": {1: 3}}) == "3"
    assert rust_template.render({"foo": {1: 3}}) == "3"


def test_render_variable_hyphen():
    template = "{{ foo-1 }}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    assert str(exc_info.value) == "Could not parse the remainder: '-1' from 'foo-1'"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    expected = """\
  × Expected a valid variable name
   ╭────
 1 │ {{ foo-1 }}
   ·    ──┬──
   ·      ╰── here
   ╰────
"""
    assert str(exc_info.value) == expected


def test_render_attribute_negative_int():
    template = "{{ foo.-1 }}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    assert str(exc_info.value) == "Could not parse the remainder: '-1' from 'foo.-1'"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    expected = """\
  × Expected a valid variable name
   ╭────
 1 │ {{ foo.-1 }}
   ·        ─┬
   ·         ╰── here
   ╰────
"""
    assert str(exc_info.value) == expected
