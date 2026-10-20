# Visible Syntax Fixtures

These inert source files are parser fixtures, not programs to execute.
From the checkout root, select one row and run both commands:

| Language override | Display command |
| --- | --- |
| `tmux set-option -p @flash-treesitter-language rust` | `cat tests/snippets/rust.rs` |
| `tmux set-option -p @flash-treesitter-language python` | `cat tests/snippets/python.py` |
| `tmux set-option -p @flash-treesitter-language javascript` | `cat tests/snippets/javascript.js` |
| `tmux set-option -p @flash-treesitter-language typescript` | `cat tests/snippets/typescript.ts` |
| `tmux set-option -p @flash-treesitter-language tsx` | `cat tests/snippets/tsx.tsx` |
| `tmux set-option -p @flash-treesitter-language json` | `cat tests/snippets/json.json` |
| `tmux set-option -p @flash-treesitter-language bash` | `cat tests/snippets/bash.sh` |
| `tmux set-option -p @flash-treesitter-language c` | `cat tests/snippets/c.c` |
| `tmux set-option -p @flash-treesitter-language cpp` | `cat tests/snippets/cpp.cpp` |
| `tmux set-option -p @flash-treesitter-language go` | `cat tests/snippets/go.go` |
| `tmux set-option -p @flash-treesitter-language lua` | `cat tests/snippets/lua.lua` |
| `tmux set-option -p @flash-treesitter-language ruby` | `cat tests/snippets/ruby.rb` |
| `tmux set-option -p @flash-treesitter-language zig` | `cat tests/snippets/zig.zig` |
| `tmux set-option -p @flash-treesitter-language php` | `cat tests/snippets/php.php` |
| `tmux set-option -p @flash-treesitter-language java` | `cat tests/snippets/java.java` |

Once the 0.2.0 bindings are installed, enter copy-mode (`prefix + [`), move
inside a nested expression, press `S`, then use `;` / `,` to expand/shrink.
Enter or a label confirms the selection. Your configured copy binding copies it.
Press `R`, type `value`, then select a syntax-range label or press Enter.
Use Esc/Ctrl+C to cancel; repeat with an existing selection to check preservation.

Explicit overrides make these checks deterministic. Auto detection may reject
short or ambiguous examples. Restore the global default with:
`tmux set-option -pu @flash-treesitter-language`.
Use a sufficiently tall pane (24+ rows) so the whole example remains visible.
