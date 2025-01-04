TEMPLATES = [
    {
        "BACKEND": "django.template.backends.django.DjangoTemplates",
        "DIRS": ["tests/templates"],
        "OPTIONS": {
            "libraries": {
                "custom_filters": "tests.templatetags.custom_filters",
                "more_filters": "tests.templatetags.more_filters",
            },
        },
    },
    {
        "BACKEND": "django_rusty_templates.RustyTemplates",
        "DIRS": ["tests/templates"],
        "NAME": "rusty",
        "OPTIONS": {
            "libraries": {
                "custom_filters": "tests.templatetags.custom_filters",
                "more_filters": "tests.templatetags.more_filters",
            },
        },
    },
]

ROOT_URLCONF = "tests.urls"
