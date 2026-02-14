from django.http import QueryDict
from django.test import RequestFactory
import pytest


factory = RequestFactory()


@pytest.mark.parametrize(
    "template,expected",
    [
        pytest.param(
            "{% querystring foo='bar' %}",
            "?foo=bar",
            id="basic_kwarg",
        ),
        pytest.param(
            "{% autoescape off %}{% querystring foo='bar' baz='qux' %}{% endautoescape %}",
            "?foo=bar&baz=qux",
            id="multiple_kwargs",
        ),
        pytest.param(
            "{% querystring foo=missing_var %}",
            "?foo=",
            id="missing_variable",
        ),
        pytest.param(
            "{% querystring foo=3 %}",
            "?foo=3",
            id="integer_variable",
        ),
        pytest.param(
            "{% querystring %}",
            "",
            id="empty",
        ),
    ],
)
def test_querystring_basic(assert_render, template, expected):
    request = factory.get("/")
    assert_render(template=template, context={}, expected=expected, request=request)


def test_querystring_override_existing(assert_render):
    template = "{% querystring foo='new' %}"
    request = factory.get("/?foo=old")
    assert_render(template=template, context={}, expected="?foo=new", request=request)


def test_querystring_existing_params_preserved(assert_render):
    template = "{% querystring %}"
    request = factory.get("/?foo=bar")
    assert_render(template=template, context={}, expected="?foo=bar", request=request)


def test_querystring_merge_with_existing(assert_render):
    template = "{% autoescape off %}{% querystring baz='new' %}{% endautoescape %}"
    request = factory.get("/?foo=bar")
    assert_render(
        template=template, context={}, expected="?foo=bar&baz=new", request=request
    )


def test_querystring_remove_key_with_none(assert_render):
    template = "{% querystring foo=None %}"
    request = factory.get("/?foo=bar&baz=qux")
    assert_render(template=template, context={}, expected="?baz=qux", request=request)


def test_querystring_remove_nonexistent_key_noop(assert_render):
    template = "{% querystring missing=None %}"
    request = factory.get("/?foo=bar")
    assert_render(template=template, context={}, expected="?foo=bar", request=request)


def test_querystring_variable_value(assert_render):
    template = "{% querystring page=next_page %}"
    request = factory.get("/")
    assert_render(
        template=template, context={"next_page": 2}, expected="?page=2", request=request
    )


def test_querystring_as_variable(assert_render):
    template = "{% querystring foo='bar' as qs %}URL{{ qs }}"
    request = factory.get("/")
    assert_render(
        template=template, context={}, expected="URL?foo=bar", request=request
    )


def test_querystring_custom_query_dict(assert_render):
    template = (
        "{% autoescape off %}{% querystring my_qd foo='bar' %}{% endautoescape %}"
    )
    qd = QueryDict("baz=qux")
    request = factory.get("/")
    assert_render(
        template=template,
        context={"my_qd": qd},
        expected="?baz=qux&foo=bar",
        request=request,
    )


def test_querystring_custom_query_dict_ignores_request(assert_render):
    template = (
        "{% autoescape off %}{% querystring my_qd foo='bar' %}{% endautoescape %}"
    )
    qd = QueryDict("baz=qux")
    request = factory.get("/?should=ignored")
    assert_render(
        template=template,
        context={"my_qd": qd},
        expected="?baz=qux&foo=bar",
        request=request,
    )


def test_querystring_iterable_value(assert_render):
    template = "{% autoescape off %}{% querystring foo=items %}{% endautoescape %}"
    request = factory.get("/")
    assert_render(
        template=template,
        context={"items": [1, 2, 3]},
        expected="?foo=1&foo=2&foo=3",
        request=request,
    )


def test_querystring_custom_query_dict_empty(assert_render):
    template = "{% querystring my_qd %}"
    qd = QueryDict("")
    assert_render(template=template, context={"my_qd": qd}, expected="")
