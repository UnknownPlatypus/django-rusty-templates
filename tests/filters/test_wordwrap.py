import pytest
from django.template import engines
from django.template.exceptions import TemplateSyntaxError


@pytest.mark.parametrize(
    "template,context,expected",
    [
        pytest.param(
            "{{ text|wordwrap:10 }}",
            {"text": "Joel is a slug"},
            "Joel is a\nslug",
            id="basic",
        ),
        pytest.param(
            "{{ text|wordwrap:20 }}",
            {"text": "short"},
            "short",
            id="short_text",
        ),
        pytest.param(
            "{{ text|wordwrap:5 }}",
            {"text": "hello"},
            "hello",
            id="exact_width",
        ),
        pytest.param(
            "{{ text|wordwrap:5 }}",
            {"text": "verylongword"},
            "verylongword",
            id="long_word_not_broken",
        ),
        pytest.param(
            "{{ text|wordwrap:10 }}",
            {"text": "line one\nline two is longer"},
            "line one\nline two\nis longer",
            id="preserves_line_breaks",
        ),
        pytest.param(
            "{{ text|wordwrap:10 }}",
            {"text": "hello    world"},
            "hello\nworld",
            id="multiple_spaces",
        ),
        pytest.param(
            "{{ text|wordwrap:10 }}",
            {"text": ""},
            "",
            id="empty_string",
        ),
        pytest.param(
            "{{ text|wordwrap:10 }}",
            {},
            "",
            id="missing_variable",
        ),
        pytest.param(
            "{{ text|wordwrap:'10' }}",
            {"text": "Joel is a slug"},
            "Joel is a\nslug",
            id="width_as_string",
        ),
        pytest.param(
            "{{ text|wordwrap:width }}",
            {"text": "Joel is a slug", "width": 10},
            "Joel is a\nslug",
            id="width_as_variable",
        ),
        pytest.param(
            "{{ text|wordwrap:10.7 }}",
            {"text": "Joel is a slug"},
            "Joel is a\nslug",
            id="width_as_float",
        ),
        pytest.param(
            "{{ text|wordwrap:1 }}",
            {"text": "a b c"},
            "a\nb\nc",
            id="width_one",
        ),
        pytest.param(
            "{{ text|wordwrap:10 }}",
            {"text": "hello world\n"},
            "hello\nworld\n",
            id="trailing_newline",
        ),
        pytest.param(
            "{{ text|wordwrap:10 }}",
            {"text": "   "},
            "   ",
            id="only_whitespace",
        ),
        pytest.param(
            "{{ text|wordwrap:10 }}",
            {"text": "hello\tworld"},
            "hello\nworld",
            id="tabs_and_spaces",
        ),
        pytest.param(
            "{{ text|wordwrap:10 }}",
            {"text": "line1\n\nline2"},
            "line1\n\nline2",
            id="multiple_newlines",
        ),
        pytest.param(
            "{{ num|wordwrap:5 }}",
            {"num": 12345},
            "12345",
            id="with_numbers",
        ),
        pytest.param(
            "{{ text|safe|wordwrap:10 }}",
            {"text": "<b>hello world</b>"},
            "<b>hello\nworld</b>",
            id="preserves_html_safety",
        ),
        pytest.param(
            "{{ text|wordwrap:15 }}",
            {"text": "The quick brown fox jumps over the lazy dog"},
            "The quick brown\nfox jumps over\nthe lazy dog",
            id="long_text",
        ),
        pytest.param(
            "{{ text|wordwrap:10 }}",
            {"text": "Hello, world! How are you?"},
            "Hello,\nworld! How\nare you?",
            id="with_punctuation",
        ),
        pytest.param(
            "{{ text|wordwrap:5 }}",
            {"text": "one two three four five"},
            "one\ntwo\nthree\nfour\nfive",
            id="single_long_line",
        ),
        pytest.param(
            "{{ text|wordwrap:12 }}",
            {"text": "a bb ccc dddd eeeee"},
            "a bb ccc\ndddd eeeee",
            id="mixed_length_words",
        ),
    ],
)
def test_wordwrap(assert_render, template, context, expected):
    assert_render(template, context, expected)


def test_wordwrap_no_argument():
    template = "{{ text|wordwrap }}"
    
    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["django"].from_string(template)
    
    assert str(exc_info.value) == "wordwrap requires 2 arguments, 1 provided"
    
    with pytest.raises(TemplateSyntaxError) as exc_info:
        engines["rusty"].from_string(template)
    
    expected = """\
  × Expected an argument
   ╭────
 1 │ {{ text|wordwrap }}
   ·         ────┬───
   ·             ╰── here
   ╰────
"""
    assert str(exc_info.value) == expected


def test_wordwrap_invalid_width_string():
    template = "{{ text|wordwrap:'invalid' }}"
    
    with pytest.raises(ValueError) as exc_info:
        engines["django"].from_string(template).render({"text": "hello"})
    
    assert str(exc_info.value) == "invalid literal for int() with base 10: 'invalid'"
    
    with pytest.raises(ValueError) as exc_info:
        engines["rusty"].from_string(template).render({"text": "hello"})
    
    expected = """\
  × Couldn't convert argument ('invalid') to integer
   ╭────
 1 │ {{ text|wordwrap:'invalid' }}
   ·                  ────┬────
   ·                      ╰── argument
   ╰────
"""
    assert str(exc_info.value) == expected


def test_wordwrap_negative_width():
    template = "{{ text|wordwrap:-5 }}"
    
    with pytest.raises(ValueError) as exc_info:
        engines["django"].from_string(template).render({"text": "hello world"})
    
    assert str(exc_info.value) == "invalid width -5 (must be > 0)"
    
    with pytest.raises(ValueError) as exc_info:
        engines["rusty"].from_string(template).render({"text": "hello world"})
    
    assert str(exc_info.value) == "invalid width 0 (must be > 0)"


def test_wordwrap_zero_width():
    template = "{{ text|wordwrap:0 }}"
    
    with pytest.raises(ValueError) as exc_info:
        engines["django"].from_string(template).render({"text": "hello world"})
    
    assert str(exc_info.value) == "invalid width 0 (must be > 0)"
    
    with pytest.raises(ValueError) as exc_info:
        engines["rusty"].from_string(template).render({"text": "hello world"})
    
    assert str(exc_info.value) == "invalid width 0 (must be > 0)"

@pytest.mark.skip("Should overflow in django")
def test_wordwrap_width_overflow():
    template = "{{ text|wordwrap:9223372036854775808 }}"
    
    with pytest.raises(OverflowError) as exc_info:
        engines["django"].from_string(template).render({"text": "hello"})
    
    assert str(exc_info.value) == "Python int too large to convert to C ssize_t"
    
    with pytest.raises(OverflowError) as exc_info:
        engines["rusty"].from_string(template).render({"text": "hello"})
    
    expected = """\
  × Integer 9223372036854775808 is too large
   ╭────
 1 │ {{ text|wordwrap:9223372036854775808 }}
   ·                  ─────────┬───────────
   ·                           ╰── here
   ╰────
"""
    assert str(exc_info.value) == expected


def test_wordwrap_width_float_overflow():
    template = "{{ text|wordwrap:1e310 }}"
    
    with pytest.raises(OverflowError) as exc_info:
        engines["django"].from_string(template).render({"text": "hello"})
    
    assert str(exc_info.value) == "cannot convert float infinity to integer"
    
    with pytest.raises(OverflowError) as exc_info:
        engines["rusty"].from_string(template).render({"text": "hello"})
    
    expected = """\
  × Couldn't convert float (inf) to integer
   ╭────
 1 │ {{ text|wordwrap:1e310 }}
   ·                  ──┬──
   ·                    ╰── here
   ╰────
"""
    assert str(exc_info.value) == expected


def test_wordwrap_width_from_python_variable():
    template = "{{ text|wordwrap:width }}"
    
    with pytest.raises(ValueError) as exc_info:
        engines["django"].from_string(template).render(
            {"text": "hello", "width": "not a number"}
        )
    
    assert str(exc_info.value) == "invalid literal for int() with base 10: 'not a number'"
    
    with pytest.raises(ValueError) as exc_info:
        engines["rusty"].from_string(template).render(
            {"text": "hello", "width": "not a number"}
        )
    
    expected = """\
  × Couldn't convert argument (not a number) to integer
   ╭────
 1 │ {{ text|wordwrap:width }}
   ·                  ──┬──
   ·                    ╰── argument
   ╰────
"""
    assert str(exc_info.value) == expected
