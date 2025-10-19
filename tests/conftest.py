import pytest
from django.template import engines


@pytest.fixture(params=["rusty", "django"])
def template_engine(request):
    """
    Parametrize tests to run against both rusty and django template engines.

    See https://docs.pytest.org/en/stable/how-to/fixtures.html#parametrizing-fixtures
    """
    return engines[request.param]


@pytest.fixture
def assert_render(template_engine):
    """
    A convenient method allowing to write concise tests rendering a template with a specific context.

    Example:
        def test_render_url_variable(assert_render):
            assert_render(template="{% url home %}", context={"home": "home"}, expected="/")
    """

    def assert_render_template(template, context, expected, request_factory=None):
        assert (
            template_engine.from_string(template).render(context, request_factory)
            == expected
        )

    return assert_render_template
