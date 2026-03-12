## e2e testing

basically, use `pytest` for normally testings

if the project is suitable for BDD testing, use `behave` to do the BDD testings.

- firstly, build the bdd testing framework on `behave`. - mapping the bdd
  language with bdd testing framework.

## Requirements

- Python >= 3.14 (see `pyproject.toml`)
- A Rust toolchain with `cargo`
- `pytest` (installed either via `uv` or `pip`)

Using `uv` (recommended):

```bash
cd e2e
uv venv .venv
uv sync
uv run pytest -v --tb=short
```

`uv` will create and manage a virtual environment based on `pyproject.toml` and
run `pytest` inside it.

Without `uv`, you can create a venv and install pytest manually:

```bash
cd e2e
python -m venv .venv
source .venv/bin/activate         # bash / zsh
# source .venv/bin/activate.fish  # fish
pip install pytest
pytest -v --tb=short
```

### data/baseline

data/baseline are in the `../data` dir.
