"""Run Django's template test suite against django-rusty-templates."""

import os
import subprocess
from pathlib import Path

SCRIPT_DIR = Path(__file__).parent
DJANGO_REPO_CACHE = SCRIPT_DIR / ".django"
PATCH_FILE = SCRIPT_DIR / "django_tests_use_rusty_templates.patch"


def log(header: str):
    print(f"\033[1;32m==> {header}\033[0m\n", flush=True)


def patch_django_test_suite():
    # Check if patch is already applied
    result = subprocess.run(
        ["git", "apply", "--check", "--quiet", str(PATCH_FILE)],
        cwd=DJANGO_REPO_CACHE,
        capture_output=True,
    )
    if result.returncode == 0:
        log("Applying patches to Django repository...")
        subprocess.run(
            ["git", "apply", str(PATCH_FILE)],
            cwd=DJANGO_REPO_CACHE,
            check=True,
        )
    else:
        log("Patches already applied or not applicable.")


def main():
    if not DJANGO_REPO_CACHE.exists():
        log(f"Cloning Django repository at {DJANGO_REPO_CACHE}...")
        subprocess.run(
            [
                "git",
                "clone",
                "--config",
                "advice.detachedHead=false",
                "--quiet",
                "--depth",
                "1",
                "--no-tags",
                "https://github.com/django/django.git",
                str(DJANGO_REPO_CACHE),
            ],
            check=True,
            cwd=SCRIPT_DIR,
        )
    else:
        log(f"Using existing Django repository at {DJANGO_REPO_CACHE}")

    patch_django_test_suite()

    log("Running Django's template test suite...")
    result = subprocess.run(
        [
            "python",
            "runtests.py",
            "--parallel=1",
            "-v=2",
            "template_tests",
            "template_loader",
        ],
        cwd=DJANGO_REPO_CACHE / "tests",
        # Ensure the cloned Django repo takes precedence over installed Django
        env={**os.environ, "PYTHONPATH": str(DJANGO_REPO_CACHE)},
    )

    log("Done !")
    return result.returncode


if __name__ == "__main__":
    raise SystemExit(main())
