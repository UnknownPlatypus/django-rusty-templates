import pytest
from django.template.loader import get_template


def render(template, context, *, using):
    template = get_template(template, using=using)
    return template.render(context)


def test_render_template():
    context = {"user": "Lily"}
    assert render("basic.txt", context, using="rusty") == render("basic.txt", context, using="django")
