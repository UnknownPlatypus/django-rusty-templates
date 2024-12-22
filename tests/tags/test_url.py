import pytest
from django.template import engines
from django.urls import NoReverseMatch


def test_render_url():
    template = "{% url 'home' %}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    expected = "/"
    assert django_template.render({}) == expected
    assert rust_template.render({}) == expected


def test_render_url_variable():
    template = "{% url home %}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    expected = "/"
    assert django_template.render({"home": "home"}) == expected
    assert rust_template.render({"home": "home"}) == expected


def test_render_url_variable_missing():
    template = "{% url home %}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    with pytest.raises(NoReverseMatch) as django_error:
        django_template.render({})

    with pytest.raises(NoReverseMatch) as rust_error:
        rust_template.render({})

    msg = "Reverse for '' not found. '' is not a valid view function or pattern name."
    assert django_error.value.args[0] == msg
    assert rust_error.value.args[0] == msg


def test_render_url_arg():
    template = "{% url 'bio' 'lily' %}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    expected = "/bio/lily/"
    assert django_template.render({}) == expected
    assert rust_template.render({}) == expected


def test_render_url_kwarg():
    template = "{% url 'bio' username='lily' %}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    expected = "/bio/lily/"
    assert django_template.render({}) == expected
    assert rust_template.render({}) == expected


def test_render_url_arg_as_variable():
    template = "{% url 'bio' 'lily' as bio %}https://example.com{{ bio }}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    expected = "https://example.com/bio/lily/"
    assert django_template.render({}) == expected
    assert rust_template.render({}) == expected


def test_render_url_kwarg_as_variable():
    template = "{% url 'bio' username='lily' as bio %}https://example.com{{ bio }}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    expected = "https://example.com/bio/lily/"
    assert django_template.render({}) == expected
    assert rust_template.render({}) == expected
