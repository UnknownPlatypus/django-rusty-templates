from django.template import engines


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
