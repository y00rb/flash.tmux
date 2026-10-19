#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
python3 "$ROOT/tests/e2e.py" EndToEnd.test_plugin_defaults_and_reload_preserve_user_bindings EndToEnd.test_actual_s_binding_repeated_copy_and_q_exit
