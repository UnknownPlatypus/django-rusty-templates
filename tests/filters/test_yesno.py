import pytest


@pytest.mark.parametrize(
    "value,expected",
    [
        # Truthy values
        (True, "yeah"),
        (1, "yeah"),
        (-1, "yeah"),
        (100, "yeah"),
        (1.5, "yeah"),
        ("hello", "yeah"),
        ("0", "yeah"),  # Non-empty string is truthy
        ([1, 2, 3], "yeah"),
        ({"a": 1}, "yeah"),
        # Falsy values
        (False, "no"),
        (0, "no"),
        (0.0, "no"),
        ("", "no"),
        ([], "no"),
        ({}, "no"),
    ],
)
def test_yesno_basic(assert_render, value, expected):
    assert_render('{{ value|yesno:"yeah,no,maybe" }}', {"value": value}, expected)


def test_yesno_with_none(assert_render):
    # Python None returns the "maybe" value
    assert_render('{{ value|yesno:"yeah,no,maybe" }}', {"value": None}, "maybe")


def test_yesno_missing_variable(assert_render):
    # Missing variables are treated as empty string (falsy), not as None
    assert_render('{{ missing|yesno:"yeah,no,maybe" }}', {}, "no")


@pytest.mark.parametrize(
    "value,expected",
    [
        (True, "yes"),
        (False, "no"),
        (1, "yes"),
        (0, "no"),
        ("", "no"),
    ],
)
def test_yesno_default_values(assert_render, value, expected):
    assert_render('{{ value|yesno }}', {"value": value}, expected)


def test_yesno_default_with_none(assert_render):
    # Python None returns the "maybe" value
    assert_render('{{ value|yesno }}', {"value": None}, "maybe")


def test_yesno_default_missing_variable(assert_render):
    # Missing variables are treated as empty string (falsy), not as None
    assert_render('{{ missing|yesno }}', {}, "no")


@pytest.mark.parametrize(
    "value,expected",
    [
        (True, "yeah"),
        (False, "no"),
        (None, "no"),  # Falls back to second value when no maybe
        ("", "no"),
    ],
)
def test_yesno_two_values(assert_render, value, expected):
    assert_render('{{ value|yesno:"yeah,no" }}', {"value": value}, expected)


def test_yesno_two_values_missing_variable(assert_render):
    assert_render('{{ missing|yesno:"yeah,no" }}', {}, "no")


@pytest.mark.parametrize(
    "template,value,expected",
    [
        ('{{ value|yesno:"yup,nope,idk" }}', True, "yup"),
        ('{{ value|yesno:"yup,nope,idk" }}', False, "nope"),
        ('{{ value|yesno:"yup,nope,idk" }}', None, "idk"),
        # Test with emojis
        ('{{ value|yesno:"👍,👎,🤷" }}', True, "👍"),
        ('{{ value|yesno:"👍,👎,🤷" }}', False, "👎"),
        ('{{ value|yesno:"👍,👎,🤷" }}', None, "🤷"),
    ],
)
def test_yesno_custom_values(assert_render, template, value, expected):
    assert_render(template, {"value": value}, expected)


@pytest.mark.parametrize(
    "value,expected",
    [
        (True, "oui"),
        (False, "non"),
        (None, "peut-être"),
    ],
)
def test_yesno_with_variable_argument(assert_render, value, expected):
    assert_render(
        '{{ value|yesno:choices }}',
        {"value": value, "choices": "oui,non,peut-être"},
        expected,
    )


@pytest.mark.parametrize(
    "template,value,expected",
    [
        # Whitespace preserved in values
        ('{{ value|yesno:" yup , nope , idk " }}', True, " yup "),
        ('{{ value|yesno:" yup , nope , idk " }}', False, " nope "),
        ('{{ value|yesno:" yup , nope , idk " }}', None, " idk "),
        # Spaces in values
        ('{{ value|yesno:"yes sir,no sir,maybe sir" }}', True, "yes sir"),
        ('{{ value|yesno:"yes sir,no sir,maybe sir" }}', False, "no sir"),
        ('{{ value|yesno:"yes sir,no sir,maybe sir" }}', None, "maybe sir"),
    ],
)
def test_yesno_with_whitespace(assert_render, template, value, expected):
    assert_render(template, {"value": value}, expected)

@pytest.mark.parametrize(
    "value,expected",
    [
        (True, ""),
        (False, ""),
        (None, "maybe"),
    ],
)
def test_yesno_empty_strings_in_argument(assert_render, value, expected):
    assert_render('{{ value|yesno:",,maybe" }}', {"value": value}, expected)


def test_yesno_chained_with_other_filters(assert_render):
    assert_render('{{ value|yesno|upper }}', {"value": True}, "YES")
    assert_render('{{ value|yesno|upper }}', {"value": False}, "NO")
    assert_render('{{ value|default:True|yesno }}', {}, "yes")


def test_yesno_with_boolean_context(assert_render):
    # Test with forloop variables which are booleans
    template = '{% for x in items %}{{ forloop.first|yesno:"first,not-first" }}{% endfor %}'
    assert_render(template, {"items": [1, 2, 3]}, "first" + "not-first" * 2)

@pytest.mark.skip("Django behaves like with 2 values and ignores the rest")
def test_yesno_four_or_more_values(assert_render):
    # Only first 3 values are used
    assert_render('{{ value|yesno:"a,b,c,d,e" }}', {"value": True}, "a")
    assert_render('{{ value|yesno:"a,b,c,d,e" }}', {"value": False}, "b")
    # None returns the "maybe" value (third value)
    assert_render('{{ value|yesno:"a,b,c,d,e" }}', {"value": None}, "c")


# Error cases


def test_yesno_invalid_argument_single_value(assert_render):
    # When argument has less than 2 values, Django returns the value as-is
    assert_render('{{ value|yesno:"only_one" }}', {"value": True}, "True")
    assert_render('{{ value|yesno:"only_one" }}', {"value": False}, "False")
    # For None, Django renders it as empty string (not "None")
    assert_render('{{ value|yesno:"only_one" }}', {"value": None}, "None")


def test_yesno_empty_argument(assert_render):
    # Empty string has 1 element after split
    assert_render('{{ value|yesno:"" }}', {"value": True}, "True")
    assert_render('{{ value|yesno:"" }}', {"value": False}, "False")
    assert_render('{{ value|yesno:"" }}', {"value": None}, "None")
