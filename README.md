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
