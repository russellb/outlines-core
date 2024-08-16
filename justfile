dev-core:
    cd outlines-core && cargo build

build-core:
    cd outlines-core && cargo build --release

dev-python:
    cd bindings/python && pip install -e .

build-python:
    cd bindings/python && python -m build
