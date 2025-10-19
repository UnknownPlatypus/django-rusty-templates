def test_lower_integer(assert_render):
    template = "{{ foo|default:3|lower }}"
    assert_render(template=template, context={}, expected="3")


def test_lower_float(assert_render):
    template = "{{ foo|default:3.7|lower }}"
    assert_render(template=template, context={}, expected="3.7")


def test_lower_bool(assert_render):
    template = "{% for x in 'ab' %}{{ forloop.first|lower }}{% endfor %}"
    assert_render(template=template, context={}, expected="truefalse")
