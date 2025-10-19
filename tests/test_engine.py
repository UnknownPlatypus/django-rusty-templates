from pathlib import Path

import pytest
from django.conf import settings
from django.template import engines, Context
from django.template.engine import Engine
from django.template.library import InvalidTemplateLibrary
from django.template.exceptions import TemplateDoesNotExist
from django_rusty_templates import RustyTemplates


def test_import_libraries_import_error():
    params = {"libraries": {"import_error": "invalid.path"}}
    expected = "Invalid template library specified. ImportError raised when trying to load 'invalid.path': No module named 'invalid'"

    with pytest.raises(InvalidTemplateLibrary) as exc_info:
        Engine(**params)

    assert str(exc_info.value) == expected

    with pytest.raises(InvalidTemplateLibrary) as exc_info:
        RustyTemplates(
            {"OPTIONS": params, "NAME": "rust", "DIRS": [], "APP_DIRS": False}
        )

    assert str(exc_info.value) == expected


def test_import_libraries_no_register():
    params = {"libraries": {"no_register": "tests"}}
    expected = "Module  tests does not have a variable named 'register'"

    with pytest.raises(InvalidTemplateLibrary) as exc_info:
        Engine(**params)

    assert str(exc_info.value) == expected

    expected = "Module 'tests' does not have a variable named 'register'"
    with pytest.raises(InvalidTemplateLibrary) as exc_info:
        RustyTemplates(
            {"OPTIONS": params, "NAME": "rust", "DIRS": [], "APP_DIRS": False}
        )

    assert str(exc_info.value) == expected


def test_import_libraries_module_error():
    params = {"libraries": {"zero_division": "tests.zero_division"}}

    with pytest.raises(ZeroDivisionError):
        Engine(**params)

    with pytest.raises(ZeroDivisionError):
        RustyTemplates(
            {"OPTIONS": params, "NAME": "rust", "DIRS": [], "APP_DIRS": False}
        )


def test_pathlib_dirs():
    engine = RustyTemplates(
        {
            "NAME": "rust",
            "OPTIONS": {},
            "DIRS": [Path(settings.BASE_DIR) / "templates"],
            "APP_DIRS": False,
        }
    )

    template = engine.get_template("basic.txt")
    assert template.render({"user": "Lily"}) == "Hello Lily!\n"


def test_select_template_first_exists():
    template = engines["rusty"].engine.select_template(
        ["basic.txt", "full_example.html"]
    )
    assert template.render({"user": "Lily"}) == "Hello Lily!\n"

    template = engines["django"].engine.select_template(
        ["basic.txt", "full_example.html"]
    )
    assert template.render(Context({"user": "Lily"})) == "Hello Lily!\n"


def test_select_template_second_exists():
    template = engines["rusty"].engine.select_template(["nonexistent.txt", "basic.txt"])
    assert template.render({"user": "Lily"}) == "Hello Lily!\n"

    template = engines["django"].engine.select_template(
        ["nonexistent.txt", "basic.txt"]
    )
    assert template.render(Context({"user": "Lily"})) == "Hello Lily!\n"


@pytest.mark.parametrize(
    "template_list,expected_error",
    [
        pytest.param([], "No template names provided", id="Empty list"),
        pytest.param(
            ["nonexistent1.txt", "nonexistent2.txt"],
            "nonexistent1.txt, nonexistent2.txt",
            id="None exist",
        ),
    ],
)
def test_select_template_errors(template_engine, template_list, expected_error):
    with pytest.raises(TemplateDoesNotExist) as exc_info:
        template_engine.engine.select_template(template_list)

    assert str(exc_info.value) == expected_error


def test_select_template_invalid(template_engine):
    with pytest.raises(UnicodeError):
        template_engine.engine.select_template(["invalid.txt"])
