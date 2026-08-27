# Contributing

## Local setup

Install Rust 1.85, Python 3.9 or newer, and maturin. Then run:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets
cargo test -p rhealpixdggs
cargo check -p rhealpixdggs-python
python -m venv .venv
source .venv/bin/activate
python -m pip install -e '.[test]'
pytest
```

## Compatibility changes

Any change to projection or cell-selection logic must add or update an upstream
golden fixture and explain numerical differences. Do not update expected cell
IDs merely to make a failing test pass.

## Performance changes

Include a Criterion benchmark or a Python benchmark that measures the affected
path. Report input size, platform, warm-up, and whether Python conversion time
is included.

## Commits and pull requests

Keep the core independent of binding-specific types. Public API changes should
update the Python stubs, README compatibility table, and roadmap as applicable.
