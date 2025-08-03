python-coverage:
    maturin develop
    pytest --cov

rust-coverage:
    #!/usr/bin/bash
    cargo llvm-cov clean --workspace
    cargo llvm-cov --no-report
    source <(cargo llvm-cov show-env --export-prefix)
    maturin develop
    pytest
    cargo llvm-cov report

rust-coverage-browser:
    #!/usr/bin/bash
    cargo llvm-cov clean --workspace
    cargo llvm-cov --no-report
    source <(cargo llvm-cov show-env --export-prefix)
    maturin develop
    pytest
    cargo llvm-cov report --open
