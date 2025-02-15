import pytest
from django.template import engines


@pytest.fixture
def rusty():
    return engines["rusty"].from_string


@pytest.fixture
def django_temp():
    return engines["django"].from_string


@pytest.fixture
def assert_render(rusty, django_temp):
    def temp(template, context, expected):
        assert django_temp(template).render(context) == expected
        assert rusty(template).render(context) == expected

    return temp
