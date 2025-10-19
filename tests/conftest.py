import pytest
from django.template import engines


@pytest.fixture
def rusty():
    return engines["rusty"].from_string


@pytest.fixture
def django_template():
    return engines["django"].from_string


@pytest.fixture
def assert_render(rusty, django_template):
    def assert_render_template(template, context, expected):
        assert django_template(template).render(context) == expected
        assert rusty(template).render(context) == expected

    return assert_render_template


@pytest.fixture(params=["rusty", "django"])
def template_engine(request):
    """
    Parametrize tests to run against both rusty and django template engines.

    See https://docs.pytest.org/en/stable/how-to/fixtures.html#parametrizing-fixtures
    """
    return engines[request.param]
