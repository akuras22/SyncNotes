#!/usr/bin/env bash
set -e
DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$DIR"

if [ ! -d venv ]; then
  python3 -m venv venv
fi
venv/bin/pip install -q -r requirements.txt
venv/bin/python app.py "$@"
