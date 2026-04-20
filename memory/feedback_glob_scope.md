---
name: Never glob outside src/ in this Rust project
description: Project-wide globs spike on .venv and other artifact dirs; always scope to src/
type: feedback
---

Never use `Glob("**/*.py")` or other project-wide patterns here. `.venv/` is in `.gitignore` and contains thousands of files.

**Why:** Repeated accidental blowups on `.venv/lib/python3.12/site-packages/...` consuming huge token counts for zero signal.

**How to apply:** Always scope globs to `src/**/*.rs` or use `find src/ -name "*.rs"`. Never glob `**/*` from the project root.
