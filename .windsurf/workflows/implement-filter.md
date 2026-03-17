---
description: Implement a new django filter in rust
---

Your role is to implement the django filters provided in the filters.rs file.
You can find the python implementation to port in /home/thibaut/workspace/django-rusty-templates/.venv/lib/python3.14/site-packages/django/template/defaultfilters.py

You should ABSOLUTELY follow every advices here:
- add en entry to FilterType
- add a matching struct with possible argument
- Update FIlter.new
- Update impl Resolve to add a match arm
- implement the trait ResolveFilter for this type
- add exhaustive tests in tests/filters, covering every django usage of this filter you know off.
- Separate test of expected errors from the others.
- use pytest.mark.parametrize to avoid repetition, using pytest.param to name the test case
- Use the `assert_render` pytest fixture in the test si that Django and rust engines are testes together
- don't add initial comment in test function that paraphrases the test name
- at the end, ask tu run the test with `maturin develop --uv && pytest -k ...`
- DONT MODIFY UNRELATED CODE
- try to keep the implementation simple and reuse existing code
