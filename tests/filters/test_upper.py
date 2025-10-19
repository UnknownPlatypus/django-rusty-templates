import pytest
from django.template import engines, TemplateSyntaxError


def test_upper_string(assert_render):
    template = "{{ var|upper }}"
    var = "foo"
    uppered = "FOO"
    assert_render(template=template, context={"var": var}, expected=uppered)


def test_upper_undefined(assert_render):
    template = "{{ var|upper }}"
    assert_render(template=template, context={}, expected="")


def test_upper_integer(assert_render):
    template = "{{ var|upper }}"
    var = "3"
    uppered = "3"
    assert_render(template=template, context={"var": var}, expected=uppered)


def test_upper_with_argument():
    template = "{{ var|upper:arg }}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    assert str(exc_info.value) == "upper requires 1 arguments, 2 provided"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    expected = """\
  × upper filter does not take an argument
   ╭────
 1 │ {{ var|upper:arg }}
   ·              ─┬─
   ·               ╰── unexpected argument
   ╰────
"""
    assert str(exc_info.value) == expected


def test_upper_unicode(assert_render):
    template = "{{ var|upper }}"
    var = "\xeb"
    uppered = "\xcb"
    assert_render(template=template, context={"var": var}, expected=uppered)


def test_upper_html(assert_render):
    template = "{{ var|upper }}"
    var = "<b>foo</b>"
    uppered = "&lt;B&gt;FOO&lt;/B&gt;"
    assert_render(template=template, context={"var": var}, expected=uppered)


def test_upper_html_safe(assert_render):
    template = "{{ var|upper|safe }}"
    var = "<b>foo</b>"
    uppered = "<B>FOO</B>"
    assert_render(template=template, context={"var": var}, expected=uppered)


def test_upper_add_strings(assert_render):
    template = "{{ var|upper|add:'bar' }}"
    var = "foo"
    uppered = "FOObar"
    assert_render(template=template, context={"var": var}, expected=uppered)


def test_upper_add_numbers(assert_render):
    template = "{{ var|upper|add:4 }}"
    var = "2"
    uppered = "6"
    assert_render(template=template, context={"var": var}, expected=uppered)
