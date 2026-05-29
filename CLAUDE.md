# Code Standards

## Linting

Always run `bash scripts/lint.sh` before committing. If it fails, fix all errors
before proceeding. Run `bash scripts/lint.sh --fix` first for auto-fixable issues
(formatting), then address remaining errors manually.

Run `bash scripts/lint.sh --lang <language>` to check only the language you just
edited (valid values: `python`, `cpp`, `rust`, `js`, `go`, `bash`).

If `.code-standards/` is missing, run:
```
bash scripts/bootstrap.sh --configs-only
```

## Naming conventions

| Language | Functions/methods | Variables | Types/classes | Constants | Private members |
|---|---|---|---|---|---|
| Python | `snake_case` | `snake_case` | `PascalCase` | `UPPER_CASE` | `_name` / `__name` |
| C++ | `camelCase` | `camelCase` | `CamelCase` | `kCamelCase` | `camelCase_` (suffix `_`) |
| Rust | `snake_case` (compiler-enforced) | `snake_case` | `PascalCase` | `SCREAMING_SNAKE_CASE` | — |
| JavaScript/TypeScript | `camelCase` | `camelCase` | `PascalCase` | `UPPER_CASE` | — |
| Go | `camelCase` (unexported) / `CamelCase` (exported) | same | same | same | — |
| Bash | `snake_case` | `snake_case` | — | `UPPER_CASE` | — |

## Language-specific rules

### Python
- Docstrings: Google convention, required on all public modules/classes/functions.
  Use triple double-quotes `"""`. Include `Args:`, `Returns:`, `Raises:` sections.
- No mutable default arguments, no wildcard imports, no relative imports across packages.

### C++
- Prefer `std::expected<T, E>` over exceptions for recoverable errors.
- Never use `dynamic_cast` — restructure with virtual functions instead.
- STL-interface functions (`begin`, `end`, `size`, `swap`, `push_back`, etc.) use
  `snake_case` so ADL and range-based for work correctly. Everything else uses
  `camelCase`.
- Header guards: `#ifndef FOO_H` (no trailing underscore). Do NOT use `#pragma once`.
- `clang-tidy` runs only on files where `compile_commands.json` is present.

### Rust
- Naming is compiler-enforced; it cannot match C++ conventions.
- Avoid `unwrap()`/`expect()` outside tests — use `?` or handle explicitly.
  Using them outside tests triggers a clippy warning.
- Doc comments: `///` on all public items; `//!` for module-level docs.

### JavaScript / TypeScript
- Use `interface` for object shapes; `type` for unions and utility types.
- Type assertions use `as` syntax (not angle-bracket).
- JSDoc (`/** ... */`) required on all exported functions and classes.

### Go
- godoc comments required on all exported symbols. Format: `// FunctionName does X.`
  (full sentence, period at end).

### Bash
- Shebang: `#!/usr/bin/env bash` on all scripts.
- All scripts start with `set -euo pipefail`.
- Use `[[` not `[`, `$(cmd)` not backticks, `local` for function variables.

## Comment discipline

1. Comments explain **why**, not what. Never restate what the code obviously does.

2. Inline comments: 2 spaces after the code, then `// comment`.

3. Block/explanatory comments: on the line before the code, indented to match.

4. Document all public APIs fully: purpose, all parameters, return value,
   errors/exceptions that can be raised.

5. Private/internal functions: add a comment when the logic is non-obvious to
   a reader who knows the language but not this codebase.

6. **When you make an implementation choice that came from user instructions or a
   design discussion not captured in the code, leave a comment explaining the
   decision.** Future agents won't have access to prior conversations.

7. TODO comments: `// TODO: description`. Use sparingly — prefer filing an issue.

## Function naming

Functions should start with a verb in all languages (e.g., `getUser`, `parseConfig`,
`validate`). Exceptions:
- Constructors (`User::User`, `new User()`)
- Predicates that read naturally as nouns (`isEmpty`, `isValid`)
- STL-interface names in C++ (`begin`, `end`, `size`)

## Style guide reference

For questions not covered here, consult the relevant Google style guide at
https://google.github.io/styleguide/ (covers Python, C++, JavaScript, TypeScript,
Go, Shell). The explicit rules above take priority over the Google guides.
