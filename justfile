dev-core:
    cd outlines-core && cargo build

build-core:
    cd outlines-core && cargo build --release

dev-python:
    cd bindings/python && pip install -e .

build-python:
    cd bindings/python && \
    ln -sf ../../outlines-core outlines-core-lib && \
    sed -i '' 's|path = "../../outlines-core"|path = "outlines-core-lib"|' Cargo.toml && \
    python -m build && \
    rm outlines-core-lib && \
    sed -i '' 's|path = "outlines-core-lib"|path = "../../outlines-core"|' Cargo.toml
