import django
from django.http import QueryDict
from django.template import engines
from inline_snapshot import snapshot
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


def test_querystring_escaped(assert_render):
    template = "{% querystring %}"
    request = factory.get("/?x=y&a=b")
    assert_render(
        template=template, context={}, expected="?x=y&amp;a=b", request=request
    )


def test_querystring_as_variable_escaped_once(assert_render):
    template = "{% querystring a=1 b=2 as qs %}{{ qs }}"
    request = factory.get("/")
    assert_render(
        template=template, context={}, expected="?a=1&amp;b=2", request=request
    )


def test_querystring_remove_all_params(assert_render):
    template = "{% querystring a=None %}"
    request = factory.get("/?a=b")
    assert_render(template=template, context={}, expected="?", request=request)


@pytest.mark.parametrize("query_dict", [None, {}])
def test_querystring_empty_params(assert_render, query_dict):
    template = "{% querystring qd %}"
    request = factory.get("/")
    assert_render(
        template=template, context={"qd": query_dict}, expected="", request=request
    )


def test_querystring_dict_value(assert_render):
    template = "{% querystring a=my_dict %}"
    request = factory.get("/")
    assert_render(
        template=template,
        context={"my_dict": {0: 0, 1: 2}},
        expected="?a=0&amp;a=1",
        request=request,
    )


def test_querystring_explicit_request_get(assert_render):
    template = "{% querystring request.GET a=2 %}"
    request = factory.get("/?a=1")
    assert_render(template=template, context={"request": request}, expected="?a=2")


def test_querystring_query_dict_keyword(assert_render):
    template = "{% querystring query_dict=my_qd a=2 %}"
    context = {"my_qd": QueryDict("a=1&b=2")}
    assert_render(template=template, context=context, expected="?a=2&amp;b=2")


def test_querystring_no_request_no_query_dict(assert_render_error):
    assert_render_error(
        template="{% querystring %}",
        context={},
        exception=AttributeError,
        django_message=snapshot("'Context' object has no attribute 'request'"),
        rusty_message=snapshot("""\
  × 'Context' object has no attribute 'request'
   ╭────
 1 │ {% querystring %}
   · ────────┬────────
   ·         ╰── here
   ╰────
"""),
    )


def test_querystring_too_many_positional_arguments(assert_parse_error):
    assert_parse_error(
        template="{% querystring a b %}",
        django_message=snapshot("'querystring' received too many positional arguments"),
        rusty_message=snapshot("""\
  × Unexpected positional argument
   ╭────
 1 │ {% querystring a b %}
   ·                  ┬
   ·                  ╰── here
   ╰────
"""),
    )


def test_querystring_positional_after_kwarg(assert_parse_error):
    assert_parse_error(
        template="{% querystring a=1 b %}",
        django_message=snapshot(
            "'querystring' received some positional argument(s) after some keyword argument(s)"
        ),
        rusty_message=snapshot("""\
  × Unexpected positional argument after keyword argument
   ╭────
 1 │ {% querystring a=1 b %}
   ·                ─┬─ ┬
   ·                 │  ╰── this positional argument
   ·                 ╰── after this keyword argument
   ╰────
"""),
    )


def test_querystring_duplicate_keyword_arguments(assert_parse_error):
    assert_parse_error(
        template="{% querystring a=1 a=2 %}",
        django_message=snapshot(
            "'querystring' received multiple values for keyword argument 'a'"
        ),
        rusty_message=snapshot("""\
  × 'querystring' received multiple values for keyword argument 'a'
   ╭────
 1 │ {% querystring a=1 a=2 %}
   ·                ─┬─ ─┬─
   ·                 │   ╰── second
   ·                 ╰── first
   ╰────
"""),
    )


def test_querystring_query_dict_positional_and_keyword():
    """
    Known divergence: Django fails at render time because `query_dict` is given twice,
    we keep the positional argument and treat the keyword as a regular parameter.
    """
    template = "{% querystring my_qd query_dict='x' %}"
    context = {"my_qd": QueryDict("a=1")}

    with pytest.raises(TypeError) as exc_info:
        engines["django"].from_string(template).render(context)
    assert str(exc_info.value) == snapshot(
        "querystring() got multiple values for argument 'query_dict'"
    )

    rendered = engines["rusty"].from_string(template).render(context)
    assert rendered == snapshot("?a=1&amp;query_dict=x")


@pytest.mark.skipif(
    django.VERSION < (6, 0),
    reason="Django 6.0 always prepends '?' and accepts several mappings as positional arguments",
)
@pytest.mark.parametrize(
    "template,context,expected",
    [
        pytest.param("{% querystring %}", {}, "?", id="empty"),
        pytest.param(
            "{% querystring my_query_dict my_dict size='S' type=None %}",
            {
                "my_query_dict": QueryDict("color=blue&size=S"),
                "my_dict": {"color": "orange", "fabric": "silk", "type": "dress"},
            },
            "?color=orange&amp;size=S&amp;fabric=silk",
            id="multiple_mappings",
        ),
    ],
)
def test_querystring_django_6_0(assert_render, template, context, expected):
    request = factory.get("/")
    assert_render(
        template=template, context=context, expected=expected, request=request
    )
