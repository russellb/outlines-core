# Outlines-core

## structure

- `outlines-core/` can be consumed as an independent low-level package
- `bindings/` contains the API exposed to other languages, in this case only python

## developing

- build only the outlines-core package `cd outlines-core && cargo build`
- dev build of python bindings `cd bindings/python && maturin develop`. If you have the conda `outlines-dev` environment activated, the outlines-core module is installed within the env automatically

There's also a [justfile](https://github.com/casey/just) for running these easier:

- `just dev-core`
- `just dev-python`

# Developer Notes

Setup a virtual environment

```bash
uv venv
source .venv/bin/activate
```

install the python bindings with

```bash
uv pip install bindings/python
```

# Testing

```bash
python -c "import outlines_core._lib;print(dir(outlines_core._lib))"
python -c "import outlines_core._lib;print(outlines_core._lib.show_me_the_flag())"
```
