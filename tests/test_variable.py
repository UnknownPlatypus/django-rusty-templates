import pytest
from django.template import engines
from django.template.exceptions import TemplateSyntaxError


def test_render_variable(assert_render):
    template = "{{ foo }}"
    assert_render(template=template, context={"foo": 3}, expected="3")


def test_render_int(assert_render):
    template = "{{ 1 }}"
    assert_render(template=template, context={"1": 2}, expected="1")


def test_render_float(assert_render):
    template = "{{ 1.2 }}"
    assert_render(template=template, context={"1.2": 2}, expected="1.2")


def test_render_negative_int(assert_render):
    template = "{{ -1 }}"
    assert_render(template=template, context={"-1": 2}, expected="-1")


def test_render_negative_float(assert_render):
    template = "{{ -1.2 }}"
    assert_render(template=template, context={"-1.2": 2}, expected="-1.2")


def test_render_attribute_int(assert_render):
    template = "{{ foo.1 }}"
    assert_render(template=template, context={"foo": {1: 3}}, expected="3")


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


def test_render_invalid_variable():
    template = "{{ & }}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    assert str(exc_info.value) == "Could not parse the remainder: '&' from '&'"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    expected = """\
  × Expected a valid variable name
   ╭────
 1 │ {{ & }}
   ·    ┬
   ·    ╰── here
   ╰────
"""
    assert str(exc_info.value) == expected


def test_render_variable_callable(assert_render):
    template = "{{ foo }}"
    assert_render(template=template, context={"foo": lambda: 3}, expected="3")


def test_render_attribute_callable(assert_render):
    template = "{{ foo.bar }}"
    assert_render(template=template, context={"foo": {"bar": lambda: 3}}, expected="3")


class DoNotCall:
    do_not_call_in_templates = True
    attr = "attribute"

    def __call__(self):
        return "called"

    def __str__(self):
        return "not called"


class AltersData:
    def __init__(self):
        self.data = 0

    def increment(self):
        self.data += 1

    increment.alters_data = True


class Both:
    do_not_call_in_templates = True
    alters_data = True

    def __init__(self):
        self.data = 0

    def __call__(self):
        self.data += 1
        return "called"

    def __str__(self):
        return "not called"


def test_render_callable_do_not_call_in_templates(assert_render):
    template = "{{ do_not_call }}"
    context = {"do_not_call": DoNotCall()}
    assert_render(template=template, context=context, expected="not called")


def test_render_callable_do_not_call_in_templates_attribute(assert_render):
    template = "{{ do_not_call.attr }}"
    context = {"do_not_call": DoNotCall()}
    assert_render(template=template, context=context, expected="attribute")


def test_render_callable_attribute_alters_data(assert_render):
    template = "{{ foo.increment }}"
    mutable = AltersData()
    context = {"foo": mutable}
    assert_render(template=template, context=context, expected="")
    assert mutable.data == 0


def test_render_callable_variable_alters_data(assert_render):
    template = "{{ increment }}"
    mutable = AltersData()
    context = {"increment": mutable.increment}
    assert_render(template=template, context=context, expected="")
    assert mutable.data == 0


def test_do_not_call_and_alters_data(assert_render):
    template = "{{ foo.data }}"
    both = Both()
    context = {"foo": both}
    assert_render(template=template, context=context, expected="0")
    assert both.data == 0
