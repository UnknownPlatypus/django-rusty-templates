from django.template import engines


def test_load_and_render_filters():
    template = "{% load custom_filters %}{{ text|cut:'ello' }}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    text = "Hello World!"
    expected = "H World!"
    assert django_template.render({"text": text}) == expected
    assert rust_template.render({"text": text}) == expected


def test_load_and_render_single_filter():
    template = "{% load cut from custom_filters %}{{ text|cut:'ello' }}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    text = "Hello World!"
    expected = "H World!"
    assert django_template.render({"text": text}) == expected
    assert rust_template.render({"text": text}) == expected


def test_load_and_render_multiple_filters():
    template = """{% load cut double multiply from custom_filters %}
{{ text|cut:'ello' }}
{{ num|double }}
{{ num|multiply }}
{{ num|multiply:4 }}
"""
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    text = "Hello World!"
    expected = "\nH World!\n4\n6\n8\n"
    assert django_template.render({"text": text, "num": 2}) == expected
    assert rust_template.render({"text": text, "num": 2}) == expected


def test_load_and_render_multiple_filter_libraries():
    template = "{% load custom_filters more_filters %}{{ num|double|square }}"
    django_template = engines["django"].from_string(template)
    rust_template = engines["rusty"].from_string(template)

    assert django_template.render({"num": 2}) == "16"
    assert rust_template.render({"num": 2}) == "16"
