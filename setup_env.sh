#!/usr/bin/env bash
set -e  # stop if any command fails

# Name for your virtual environment folder
VENV_DIR=".venv"

# 1. Create venv if it doesn't exist
if [ ! -d "$VENV_DIR" ]; then
    echo "Creating virtual environment in $VENV_DIR..."
    python3 -m venv "$VENV_DIR"
else
    echo "Virtual environment already exists in $VENV_DIR"
fi

# 2. Activate the venv
echo "Activating virtual environment..."
# shellcheck disable=SC1090
source "$VENV_DIR/bin/activate"

# 3. Upgrade pip
echo "Upgrading pip..."
pip install --upgrade pip

# 4. Install required packages
echo "Installing required packages..."
pip install matplotlib

echo
echo "✅ Environment setup complete."
echo "To activate it in the future, run:"
echo "    source $VENV_DIR/bin/activate"
