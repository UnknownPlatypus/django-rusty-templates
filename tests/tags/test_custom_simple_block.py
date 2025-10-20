import pytest
from django.template import engines
from django.template.base import VariableDoesNotExist


def test_simple_block_tag_repeat(assert_render):
    template = "{% load repeat from custom_tags %}{% repeat 5 %}foo{% endrepeat %}"
    assert_render(template=template, context={}, expected="foofoofoofoofoo")


def test_simple_block_tag_repeat_as(assert_render):
    template = "{% load repeat from custom_tags %}{% repeat 2 as bar %}foo{% endrepeat %}{{ bar }}{{ bar|upper }}"
    assert_render(template=template, context={}, expected="foofooFOOFOO")


def test_with_block(assert_render):
    template = "{% load with_block from custom_tags %}{% with_block var='name' %}{{ user }}{% end_with_block %}{{ name|lower }}"
    context = {"user": "Lily"}
    assert_render(template=template, context=context, expected="lily")


def test_simple_block_tag_missing_context(assert_parse_error):
    template = "{% load missing_context_block from invalid_tags %}{% missing_context_block %}{% end_missing_context_block %}"
    django_message = "'missing_context_block' is decorated with takes_context=True so it must have a first argument of 'context' and a second argument of 'content'"
    rusty_message = """\
  × 'missing_context_block' is decorated with takes_context=True so it must
  │ have a first argument of 'context' and a second argument of 'content'
   ╭────
 1 │ {% load missing_context_block from invalid_tags %}{% missing_context_block %}{% end_missing_context_block %}
   ·         ──────────┬──────────
   ·                   ╰── loaded here
   ╰────
"""
    assert_parse_error(
        template=template, django_message=django_message, rusty_message=rusty_message
    )


def test_simple_block_tag_missing_content(assert_parse_error):
    template = "{% load missing_content_block from invalid_tags %}{% missing_content_block %}{% end_missing_content_block %}"
    django_message = "'missing_content_block' must have a first argument of 'content'"
    rusty_message = """\
  × 'missing_content_block' must have a first argument of 'content'
   ╭────
 1 │ {% load missing_content_block from invalid_tags %}{% missing_content_block %}{% end_missing_content_block %}
   ·         ──────────┬──────────
   ·                   ╰── loaded here
   ╰────
"""
    assert_parse_error(
        template=template, django_message=django_message, rusty_message=rusty_message
    )


def test_simple_block_tag_missing_content_takes_context(assert_parse_error):
    template = "{% load missing_content_block_with_context from invalid_tags %}{% missing_content_block_with_context %}{% end_missing_content_block_with_context %}"
    django_message = "'missing_content_block_with_context' is decorated with takes_context=True so it must have a first argument of 'context' and a second argument of 'content'"
    rusty_message = """\
  × 'missing_content_block_with_context' is decorated with takes_context=True
  │ so it must have a first argument of 'context' and a second argument of
  │ 'content'
   ╭────
 1 │ {% load missing_content_block_with_context from invalid_tags %}{% missing_content_block_with_context %}{% end_missing_content_block_with_context %}
   ·         ─────────────────┬────────────────
   ·                          ╰── loaded here
   ╰────
"""
    assert_parse_error(
        template=template, django_message=django_message, rusty_message=rusty_message
    )


def test_simple_block_tag_missing_end_tag(assert_parse_error):
    template = "{% load repeat from custom_tags %}{% repeat 3 %}"
    django_message = "Unclosed tag on line 1: 'repeat'. Looking for one of: endrepeat."
    rusty_message = """\
  × Unclosed 'repeat' tag. Looking for one of: endrepeat
   ╭────
 1 │ {% load repeat from custom_tags %}{% repeat 3 %}
   ·                                   ───────┬──────
   ·                                          ╰── started here
   ╰────
"""
    assert_parse_error(
        template=template, django_message=django_message, rusty_message=rusty_message
    )


def test_simple_block_tag_end_tag_only(assert_parse_error):
    template = "{% load repeat from custom_tags %}{% endrepeat %}"
    django_message = "Invalid block tag on line 1: 'endrepeat'. Did you forget to register or load this tag?"
    rusty_message = """\
  × Unexpected tag endrepeat
   ╭────
 1 │ {% load repeat from custom_tags %}{% endrepeat %}
   ·                                   ───────┬───────
   ·                                          ╰── unexpected tag
   ╰────
"""
    assert_parse_error(
        template=template, django_message=django_message, rusty_message=rusty_message
    )


def test_simple_block_tag_missing_argument():
    template = "{% load repeat from custom_tags %}{% repeat five %}{% endrepeat %}"

    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    with pytest.raises(TypeError) as exc_info:
        django_template.render({})

    assert str(exc_info.value) == "can't multiply sequence by non-int of type 'str'"

    with pytest.raises(TypeError) as exc_info:
        rust_template.render({})

    assert (
        str(exc_info.value)
        == """\
  × can't multiply sequence by non-int of type 'str'
   ╭────
 1 │ {% load repeat from custom_tags %}{% repeat five %}{% endrepeat %}
   ·                                   ────────┬────────
   ·                                           ╰── here
   ╰────
"""
    )


def test_simple_block_tag_invalid_argument():
    template = "{% load repeat from custom_tags %}{% repeat five|default:five %}{% endrepeat %}"

    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    with pytest.raises(VariableDoesNotExist) as exc_info:
        django_template.render({})

    assert (
        str(exc_info.value)
        == "Failed lookup for key [five] in [{'True': True, 'False': False, 'None': None}, {}]"
    )

    with pytest.raises(VariableDoesNotExist) as exc_info:
        rust_template.render({})

    assert (
        str(exc_info.value)
        == """\
  × Failed lookup for key [five] in {"False": False, "None": None, "True":
  │ True}
   ╭────
 1 │ {% load repeat from custom_tags %}{% repeat five|default:five %}{% endrepeat %}
   ·                                                          ──┬─
   ·                                                            ╰── key
   ╰────
"""
    )


def test_simple_block_tag_argument_syntax_error(assert_parse_error):
    template = "{% load repeat from custom_tags %}{% repeat a= %}{% endrepeat %}"
    django_message = "Could not parse the remainder: '=' from 'a='"
    rusty_message = """\
  × Incomplete keyword argument
   ╭────
 1 │ {% load repeat from custom_tags %}{% repeat a= %}{% endrepeat %}
   ·                                             ─┬
   ·                                              ╰── here
   ╰────
"""
    assert_parse_error(
        template=template, django_message=django_message, rusty_message=rusty_message
    )


def test_simple_block_tag_content_render_error():
    template = "{% load repeat from custom_tags %}{% repeat 2 %}{{ foo|default:bar }}{% endrepeat %}"

    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    with pytest.raises(VariableDoesNotExist) as exc_info:
        django_template.render({})

    error = "Failed lookup for key [bar] in [{'True': True, 'False': False, 'None': None}, {}]"
    assert str(exc_info.value) == error

    with pytest.raises(VariableDoesNotExist) as exc_info:
        rust_template.render({})

    error = """\
  × Failed lookup for key [bar] in {"False": False, "None": None, "True":
  │ True}
   ╭────
 1 │ {% load repeat from custom_tags %}{% repeat 2 %}{{ foo|default:bar }}{% endrepeat %}
   ·                                                                ─┬─
   ·                                                                 ╰── key
   ╰────
"""
    assert str(exc_info.value) == error
