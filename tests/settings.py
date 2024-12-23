TEMPLATES = [
    {
        "BACKEND": "django.template.backends.django.DjangoTemplates",
        "DIRS": ["tests/templates"],
    },
    {
        "BACKEND": "django_rusty_templates.RustyTemplates",
        "DIRS": ["tests/templates"],
        "NAME": "rusty",
    },
]

ROOT_URLCONF = "tests.urls"
