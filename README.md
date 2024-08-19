# Outlines-core

## structure

- `outlines-core/` can be consumed as an independent low-level package
- `bindings/` contains the API exposed to other languages, in this case only python

## developing

There's a [justfile](https://github.com/casey/just) for most dev & build tasks

- build only the outlines-core rust crate `cd outlines-core && cargo build`
- install an editable pip package with the recepie `just dev-python` which is:
```bash
cd bindings/python && pip install -e .
```
- to build the python package, run `just build-python`, which is equivalent to:
```bash
cd bindings/python && \
ln -sf ../../outlines-core outlines-core-lib && \
sed -i '' 's|path = "../../outlines-core"|path = "outlines-core-lib"|' Cargo.toml && \
python -m build && \
rm outlines-core-lib && \
sed -i '' 's|path = "outlines-core-lib"|path = "../../outlines-core"|' Cargo.toml
```

### Developer Notes

- Setup a virtual environment before running the build or dev commands

- If you get the `LookupError: setuptools-scm was unable to detect version for...` error, set the env var `SETUPTOOLS_SCM_PRETEND_VERSION=0.1.0-dev` before running the build or dev command.
