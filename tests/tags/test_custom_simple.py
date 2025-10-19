from datetime import datetime
from zoneinfo import ZoneInfo

import pytest
from django.template import engines
from django.template.base import VariableDoesNotExist
from django.template.exceptions import TemplateSyntaxError
from django.test import RequestFactory


def test_simple_tag_double(assert_render):
    template = "{% load double from custom_tags %}{% double 3 %}"
    assert_render(template=template, context={}, expected="6")


def test_simple_tag_double_kwarg(assert_render):
    template = "{% load double from custom_tags %}{% double value=3 %}"
    assert_render(template=template, context={}, expected="6")


def test_simple_tag_double_missing_variable(assert_render):
    template = "{% load double from custom_tags %}{% double foo %}"
    assert_render(template=template, context={}, expected="")


def test_simple_tag_multiply_missing_variables():
    template = "{% load multiply from custom_tags %}{% multiply foo bar eggs %}"

    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    with pytest.raises(TypeError) as exc_info:
        django_template.render({})

    error = str(exc_info.value)
    assert error == "can't multiply sequence by non-int of type 'str'"

    with pytest.raises(TypeError) as exc_info:
        rust_template.render({})

    error = str(exc_info.value)
    assert (
        error
        == """\
  × can't multiply sequence by non-int of type 'str'
   ╭────
 1 │ {% load multiply from custom_tags %}{% multiply foo bar eggs %}
   ·                                     ─────────────┬─────────────
   ·                                                  ╰── here
   ╰────
"""
    )


def test_simple_tag_kwargs(assert_render):
    template = "{% load table from custom_tags %}{% table foo='bar' spam=1 %}"
    assert_render(template=template, context={}, expected="foo-bar\nspam-1")


def test_simple_tag_positional_and_kwargs(assert_render):
    template = "{% load multiply from custom_tags %}{% multiply 3 b=2 c=4 %}"
    assert_render(template=template, context={}, expected="24")


def test_simple_tag_double_as_variable(assert_render):
    template = (
        "{% load double from custom_tags %}{% double 3 as foo %}{{ foo }}{{ foo }}"
    )
    assert_render(template=template, context={}, expected="66")


def test_simple_tag_double_kwarg_as_variable(assert_render):
    template = "{% load double from custom_tags %}{% double value=3 as foo %}{{ foo }}"
    assert_render(template=template, context={}, expected="6")


def test_simple_tag_as_variable_after_default(assert_render):
    template = "{% load invert from custom_tags %}{% invert as foo %}{{ foo }}"
    assert_render(template=template, context={}, expected="0.5")


def test_simple_tag_varargs(assert_render):
    template = "{% load combine from custom_tags %}{% combine 2 3 4 as foo %}{{ foo }}"
    assert_render(template=template, context={}, expected="9")


def test_simple_tag_varargs_with_kwarg(assert_render):
    template = "{% load combine from custom_tags %}{% combine 2 3 4 operation='multiply' as foo %}{{ foo }}"
    assert_render(template=template, context={}, expected="24")


def test_simple_tag_keyword_only(assert_render):
    template = "{% load list from custom_tags %}{% list items header='Items' %}"
    expected = """\
# Items
* 1
* 2
* 3"""
    assert_render(template=template, context={"items": [1, 2, 3]}, expected=expected)


def test_simple_tag_takes_context(assert_render):
    template = "{% load request_path from custom_tags %}{% request_path %}{{ bar }}"

    factory = RequestFactory()
    request = factory.get("/foo/")

    assert_render(
        template=template,
        context={"bar": "bar"},
        request=request,
        expected="/foo/bar",
    )


def test_simple_tag_takes_context_context_reference_held(template_engine):
    template = "{% load request_path from invalid_tags %}{% request_path %}{{ bar }}"
    template_obj = template_engine.from_string(template)

    factory = RequestFactory()
    request = factory.get("/foo/")
    assert template_obj.render({"bar": "bar"}, request) == "/foo/bar"


def test_simple_tag_takes_context_get_variable(assert_render):
    template = """\
{% load greeting from custom_tags %}{% greeting 'Charlie' %}
{% for user in users %}{% greeting 'Lily' %}{% endfor %}
{% greeting 'George' %}"""
    expected = """\
Hello Charlie from Django!
Hello Lily from Rusty Templates!
Hello George from Django!"""
    assert_render(
        template=template, context={"users": ["Rusty Templates"]}, expected=expected
    )


def test_simple_tag_takes_context_getitem(assert_render):
    template = "{% load local_time from custom_tags %}{% local_time dt %}"
    source_time = datetime(2025, 8, 31, 9, 14, tzinfo=ZoneInfo("Europe/London"))
    destination_timezone = ZoneInfo("Australia/Melbourne")
    context = {"dt": source_time, "timezone": destination_timezone}
    expected = str(source_time.astimezone(destination_timezone))
    assert_render(template=template, context=context, expected=expected)


def test_simple_tag_takes_context_setitem(assert_render):
    template = "{% load counter from custom_tags %}{% counter %}{{ count }}"
    assert_render(template=template, context={}, expected="1")


def test_simple_tag_takes_context_setitem_in_loop(assert_render):
    template = "{% load counter from custom_tags %}{% for item in items %}{% if item %}{% counter %}{% endif %}{{ count }}{% endfor %}{{ count }}"
    assert_render(template=template, context={"items": [1, 0, 4, 0]}, expected="1122")


def test_simple_tag_takes_context_getitem_missing():
    template = "{% load local_time from custom_tags %}{% local_time dt %}"

    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    source_time = datetime(2025, 8, 31, 9, 14, tzinfo=ZoneInfo("Europe/London"))
    context = {"dt": source_time}
    with pytest.raises(KeyError) as exc_info:
        django_template.render(context)

    assert str(exc_info.value) == "'timezone'"

    with pytest.raises(KeyError) as exc_info:
        rust_template.render(context)

    expected = """\
  × 'timezone'
   ╭────
 1 │ {% load local_time from custom_tags %}{% local_time dt %}
   ·                                       ─────────┬─────────
   ·                                                ╰── here
   ╰────
"""
    assert str(exc_info.value) == expected


def test_simple_tag_positional_after_kwarg():
    template = "{% load double from custom_tags %}{% double value=3 foo %}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    assert (
        str(exc_info.value)
        == "'double' received some positional argument(s) after some keyword argument(s)"
    )

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    assert (
        str(exc_info.value)
        == """\
  × Unexpected positional argument after keyword argument
   ╭────
 1 │ {% load double from custom_tags %}{% double value=3 foo %}
   ·                                             ───┬─── ─┬─
   ·                                                │     ╰── this positional argument
   ·                                                ╰── after this keyword argument
   ╰────
"""
    )


def test_simple_tag_too_many_positional_arguments():
    template = "{% load double from custom_tags %}{% double value foo %}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    assert str(exc_info.value) == "'double' received too many positional arguments"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    assert (
        str(exc_info.value)
        == """\
  × Unexpected positional argument
   ╭────
 1 │ {% load double from custom_tags %}{% double value foo %}
   ·                                                   ─┬─
   ·                                                    ╰── here
   ╰────
"""
    )


def test_simple_tag_invalid_keyword_argument():
    template = "{% load double from custom_tags %}{% double foo=bar %}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    assert str(exc_info.value) == "'double' received unexpected keyword argument 'foo'"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    assert (
        str(exc_info.value)
        == """\
  × Unexpected keyword argument
   ╭────
 1 │ {% load double from custom_tags %}{% double foo=bar %}
   ·                                             ───┬───
   ·                                                ╰── here
   ╰────
"""
    )


def test_simple_tag_missing_argument():
    template = "{% load double from custom_tags %}{% double %}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    assert (
        str(exc_info.value)
        == "'double' did not receive value(s) for the argument(s): 'value'"
    )

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    assert (
        str(exc_info.value)
        == """\
  × 'double' did not receive value(s) for the argument(s): 'value'
   ╭────
 1 │ {% load double from custom_tags %}{% double %}
   ·                                            ▲
   ·                                            ╰── here
   ╰────
"""
    )


def test_simple_tag_missing_arguments():
    template = "{% load multiply from custom_tags %}{% multiply %}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    assert (
        str(exc_info.value)
        == "'multiply' did not receive value(s) for the argument(s): 'a', 'b', 'c'"
    )

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    assert (
        str(exc_info.value)
        == """\
  × 'multiply' did not receive value(s) for the argument(s): 'a', 'b', 'c'
   ╭────
 1 │ {% load multiply from custom_tags %}{% multiply %}
   ·                                                ▲
   ·                                                ╰── here
   ╰────
"""
    )


def test_simple_tag_missing_arguments_with_kwarg():
    template = "{% load multiply from custom_tags %}{% multiply b=2 %}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    assert (
        str(exc_info.value)
        == "'multiply' did not receive value(s) for the argument(s): 'a', 'c'"
    )

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    assert (
        str(exc_info.value)
        == """\
  × 'multiply' did not receive value(s) for the argument(s): 'a', 'c'
   ╭────
 1 │ {% load multiply from custom_tags %}{% multiply b=2 %}
   ·                                                 ─┬─
   ·                                                  ╰── here
   ╰────
"""
    )


def test_simple_tag_duplicate_keyword_arguments():
    template = "{% load multiply from custom_tags %}{% multiply a=1 b=2 c=3 b=4 %}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    assert (
        str(exc_info.value)
        == "'multiply' received multiple values for keyword argument 'b'"
    )

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    assert (
        str(exc_info.value)
        == """\
  × 'multiply' received multiple values for keyword argument 'b'
   ╭────
 1 │ {% load multiply from custom_tags %}{% multiply a=1 b=2 c=3 b=4 %}
   ·                                                     ─┬─     ─┬─
   ·                                                      │       ╰── second
   ·                                                      ╰── first
   ╰────
"""
    )


def test_simple_tag_keyword_as_multiple_variables():
    template = "{% load double from custom_tags %}{% double value=1 as foo bar %}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    assert (
        str(exc_info.value)
        == "'double' received some positional argument(s) after some keyword argument(s)"
    )

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    assert (
        str(exc_info.value)
        == """\
  × Unexpected positional argument after keyword argument
   ╭────
 1 │ {% load double from custom_tags %}{% double value=1 as foo bar %}
   ·                                             ───┬─── ─┬
   ·                                                │     ╰── this positional argument
   ·                                                ╰── after this keyword argument
   ╰────
"""
    )


def test_simple_tag_positional_as_multiple_variables():
    template = "{% load double from custom_tags %}{% double value as foo bar %}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    assert str(exc_info.value) == "'double' received too many positional arguments"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    assert (
        str(exc_info.value)
        == """\
  × Unexpected positional argument
   ╭────
 1 │ {% load double from custom_tags %}{% double value as foo bar %}
   ·                                                   ─┬
   ·                                                    ╰── here
   ╰────
"""
    )


def test_simple_tag_positional_as_multiple_variables_with_default():
    template = "{% load invert from custom_tags %}{% invert as foo bar %}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    assert str(exc_info.value) == "'invert' received too many positional arguments"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    assert (
        str(exc_info.value)
        == """\
  × Unexpected positional argument
   ╭────
 1 │ {% load invert from custom_tags %}{% invert as foo bar %}
   ·                                                ─┬─
   ·                                                 ╰── here
   ╰────
"""
    )


def test_simple_tag_keyword_missing_target_variable():
    template = "{% load double from custom_tags %}{% double value=1 as %}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    assert (
        str(exc_info.value)
        == "'double' received some positional argument(s) after some keyword argument(s)"
    )

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    assert (
        str(exc_info.value)
        == """\
  × Unexpected positional argument after keyword argument
   ╭────
 1 │ {% load double from custom_tags %}{% double value=1 as %}
   ·                                             ───┬─── ─┬
   ·                                                │     ╰── this positional argument
   ·                                                ╰── after this keyword argument
   ╰────
"""
    )


def test_simple_tag_positional_missing_target_variable():
    template = "{% load double from custom_tags %}{% double value as %}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    assert str(exc_info.value) == "'double' received too many positional arguments"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    assert (
        str(exc_info.value)
        == """\
  × Unexpected positional argument
   ╭────
 1 │ {% load double from custom_tags %}{% double value as %}
   ·                                                   ─┬
   ·                                                    ╰── here
   ╰────
"""
    )


def test_simple_tag_incomplete_keyword_argument():
    template = "{% load double from custom_tags %}{% double value= %}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    assert str(exc_info.value) == "Could not parse the remainder: '=' from 'value='"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    assert (
        str(exc_info.value)
        == """\
  × Incomplete keyword argument
   ╭────
 1 │ {% load double from custom_tags %}{% double value= %}
   ·                                             ───┬──
   ·                                                ╰── here
   ╰────
"""
    )


def test_simple_tag_invalid_filter():
    template = "{% load double from custom_tags %}{% double foo|bar %}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    assert str(exc_info.value) == "Invalid filter: 'bar'"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    assert (
        str(exc_info.value)
        == """\
  × Invalid filter: 'bar'
   ╭────
 1 │ {% load double from custom_tags %}{% double foo|bar %}
   ·                                                 ─┬─
   ·                                                  ╰── here
   ╰────
"""
    )


def test_simple_tag_invalid_filter_in_keyword_argument():
    template = "{% load double from custom_tags %}{% double value=foo|bar %}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    assert str(exc_info.value) == "Invalid filter: 'bar'"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    assert (
        str(exc_info.value)
        == """\
  × Invalid filter: 'bar'
   ╭────
 1 │ {% load double from custom_tags %}{% double value=foo|bar %}
   ·                                                       ─┬─
   ·                                                        ╰── here
   ╰────
"""
    )


def test_simple_tag_render_error():
    template = "{% load custom_tags %}{% combine operation='divide' %}"

    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    with pytest.raises(RuntimeError) as exc_info:
        django_template.render({})

    assert str(exc_info.value) == "Unknown operation"

    with pytest.raises(RuntimeError) as exc_info:
        rust_template.render({})

    assert (
        str(exc_info.value)
        == """\
  × Unknown operation
   ╭────
 1 │ {% load custom_tags %}{% combine operation='divide' %}
   ·                       ────────────────┬───────────────
   ·                                       ╰── here
   ╰────
"""
    )


def test_simple_tag_argument_error():
    template = "{% load double from custom_tags %}{% double foo|default:bar %}"

    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    with pytest.raises(VariableDoesNotExist) as exc_info:
        django_template.render({})

    expected = "Failed lookup for key [bar] in [{'True': True, 'False': False, 'None': None}, {}]"
    assert str(exc_info.value) == expected

    with pytest.raises(VariableDoesNotExist) as exc_info:
        rust_template.render({})

    expected = """\
  × Failed lookup for key [bar] in {"False": False, "None": None, "True":
  │ True}
   ╭────
 1 │ {% load double from custom_tags %}{% double foo|default:bar %}
   ·                                                         ─┬─
   ·                                                          ╰── key
   ╰────
"""
    assert str(exc_info.value) == expected


def test_simple_tag_keyword_argument_error():
    template = "{% load double from custom_tags %}{% double value=foo|default:bar %}"

    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    with pytest.raises(VariableDoesNotExist) as exc_info:
        django_template.render({})

    expected = "Failed lookup for key [bar] in [{'True': True, 'False': False, 'None': None}, {}]"
    assert str(exc_info.value) == expected

    with pytest.raises(VariableDoesNotExist) as exc_info:
        rust_template.render({})

    expected = """\
  × Failed lookup for key [bar] in {"False": False, "None": None, "True":
  │ True}
   ╭────
 1 │ {% load double from custom_tags %}{% double value=foo|default:bar %}
   ·                                                               ─┬─
   ·                                                                ╰── key
   ╰────
"""
    assert str(exc_info.value) == expected


def test_simple_tag_missing_keyword_argument():
    template = "{% load list from custom_tags %}{% list %}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    assert (
        str(exc_info.value)
        == "'list' did not receive value(s) for the argument(s): 'items', 'header'"
    )

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    expected = """\
  × 'list' did not receive value(s) for the argument(s): 'items', 'header'
   ╭────
 1 │ {% load list from custom_tags %}{% list %}
   ·                                        ▲
   ·                                        ╰── here
   ╰────
"""
    assert str(exc_info.value) == expected


def test_simple_tag_missing_context():
    template = "{% load missing_context from invalid_tags %}{% missing_context %}"

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)

    assert (
        str(exc_info.value)
        == "'missing_context' is decorated with takes_context=True so it must have a first argument of 'context'"
    )

    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)

    assert (
        str(exc_info.value)
        == """\
  × 'missing_context' is decorated with takes_context=True so it must have a
  │ first argument of 'context'
   ╭────
 1 │ {% load missing_context from invalid_tags %}{% missing_context %}
   ·         ───────┬───────
   ·                ╰── loaded here
   ╰────
"""
    )
