# Gsh Next Plan

Gsh is the ArcOS shell. Current implementation has many commands but profile behavior and TTY handling need robustness.

## Priorities
1. Stable interactive mode over PTY.
2. Profile fallback:
   - source `/etc/gshrc` only if readable;
   - guard optional binary commands.
3. Reliable line editor / history / backspace / Enter:
   - ensure key mapping for Tab, Enter, arrows.
4. Build a launch environment:
   - usable through `arcos-term`.
5. Standard basic commands:
   - `cat`, `cd`, `cp`, etc.
   - robust errors and exit code.

## Testing
- PTY smoke test: shell echoes command output.
- backspace test.
- EOF exits cleanly.
- history survives process lifetime.

## Integration goal
When launched as a terminal child, prompt appears and commands execute.