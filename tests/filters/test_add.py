import pytest
from django.template import engines
from django.template.base import VariableDoesNotExist


def test_add_integers():
    template = "{{ foo|add:3 }}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    assert django_template.render({"foo": 2}) == "5"
    assert rust_template.render({"foo": 2}) == "5"


def test_add_no_variable():
    template = "{{ foo|add:3 }}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    assert django_template.render({}) == ""
    assert rust_template.render({}) == ""


def test_add_no_argument():
    template = "{{ foo|add:bar }}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    with pytest.raises(VariableDoesNotExist) as exc_info:
        django_template.render({"foo": 1})

    assert str(exc_info.value) == "Failed lookup for key [bar] in [{'True': True, 'False': False, 'None': None}, {'foo': 1}]"

    with pytest.raises(VariableDoesNotExist) as exc_info:
        rust_template.render({"foo": 1})

    assert str(exc_info.value) == """
  × Failed lookup for key [bar] in {"foo": 1}
   ╭────
 1 │ {{ foo|add:bar }}
   ·            ─┬─
   ·             ╰── key
   ╰────
"""


def test_add_integer_strings():
    template = "{{ foo|add:'3' }}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    assert django_template.render({"foo": "2"}) == "5"
    assert rust_template.render({"foo": "2"}) == "5"
