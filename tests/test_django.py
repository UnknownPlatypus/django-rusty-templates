from pathlib import Path
from textwrap import dedent

import pytest
from django.template import engines
from django.template.exceptions import TemplateSyntaxError
from django.template.loader import get_template


def render(template, context, *, using):
    template = get_template(template, using=using)
    return template.render(context)


def test_render_template():
    context = {"user": "Lily"}
    assert render("basic.txt", context, using="rusty") == render("basic.txt", context, using="django")


def test_parse_error():
    with pytest.raises(TemplateSyntaxError) as excinfo:
        get_template("parse_error.txt", using="rusty")

    template_dir = Path("tests/templates").absolute()
    expected = """\
  × Empty variable tag
   ╭─[%s/parse_error.txt:1:28]
 1 │ This is an empty variable: {{ }}
   ·                            ──┬──
   ·                              ╰── here
   ╰────
""" % template_dir
    assert str(excinfo.value) == expected


def test_parse_error_from_string():
    rusty_engine = engines["rusty"]

    template = """
This is an invalid filter name: {{ variable|'invalid'|title }}
"""

    with pytest.raises(TemplateSyntaxError) as excinfo:
        rusty_engine.from_string(template)

    expected = """\
  × Expected a valid filter name
   ╭─[2:45]
 1 │\x20
 2 │ This is an invalid filter name: {{ variable|'invalid'|title }}
   ·                                             ────┬────
   ·                                                 ╰── here
   ╰────
"""
    assert str(excinfo.value) == expected
