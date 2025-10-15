from pathlib import Path

import pytest
from django.conf import settings
from django.template.engine import Engine
from django.template.library import InvalidTemplateLibrary
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

    context = {"user": "Lily"}
    expected = "Hello Lily!\n"

    template = engine.get_template("basic.txt")
    assert template.render(context) == expected


@pytest.mark.parametrize(
    "loaders,template_name,expected",
    [
        pytest.param(
            [
                "django.template.loaders.filesystem.Loader",
                "django.template.loaders.app_directories.Loader",
            ],
            "basic.txt",
            "Hello Lily!\n",
            id="Loader priority",
        ),
        pytest.param(
            [
                (
                    "django.template.loaders.cached.Loader",
                    [
                        "django.template.loaders.filesystem.Loader",
                        "django.template.loaders.app_directories.Loader",
                    ],
                ),
            ],
            "basic.txt",
            "Hello Lily!\n",
            id="Cached loader",
        ),
        pytest.param(
            [
                (
                    "django.template.loaders.cached.Loader",
                    [
                        ("django.template.loaders.filesystem.Loader", ["tests", "app"]),
                        "django.template.loaders.app_directories.Loader",
                    ],
                ),
            ],
            "basic.txt",
            "Hello Lily!\n",
            id="Cached loader + Filesystem Loader with dirs",
        ),
        pytest.param(
            [
                (
                    "django.template.loaders.locmem.Loader",
                    {"index.html": "Welcome {{ user }}!"},
                ),
            ],
            "index.html",
            "Welcome Lily!",
            id="Locmem Loader",
        ),
    ],
)
def test_loader_configurations(loaders, template_name, expected):
    engine = RustyTemplates(
        {
            "OPTIONS": {"loaders": loaders},
            "NAME": "rust",
            "DIRS": [],
            "APP_DIRS": False,
        }
    )

    context = {"user": "Lily"}
    template = engine.get_template(template_name)
    assert template.render(context) == expected
