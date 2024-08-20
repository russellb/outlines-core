#!/bin/bash

# Build sdist and wheel
python -m pip install -U pip
python -m pip install build
cd bindings/python
ln -sf ../../outlines-core outlines-core-lib
sed -i '' 's|path = "../../outlines-core"|path = "outlines-core-lib"|' Cargo.toml
python -m build
cd ../..


# Check sdist install and imports
mkdir -p test-sdist
cd test-sdist
python -m venv venv-sdist
venv-sdist/bin/python -m pip install ../bindings/python/dist/outlines_core-*.tar.gz
venv-sdist/bin/python -c "import outlines_core"
cd ..

# Check wheel install and imports
mkdir -p test-wheel
cd test-wheel
python -m venv venv-wheel
venv-wheel/bin/python -m pip install ../bindings/python/dist/outlines_core-*.whl
venv-wheel/bin/python -c "import outlines_core"
cd ..
