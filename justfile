dev-core:
    cd outlines-core && cargo build

dev-python:
    cd bindings/python && maturin develop
