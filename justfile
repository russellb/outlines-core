dev-core:
    cd outlines-core && cargo build

build-core:
    cd outlines-core && cargo build --release

dev-python:
    cd bindings/python && maturin develop

build-python:
    cd bindings/python && maturin build --release
