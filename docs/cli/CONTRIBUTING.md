# Contributing to the Dali CLI

## Module ownership

- src/main.rs owns process startup and top-level error reporting.
- src/commands/mod.rs owns command dispatch and shared argument helpers.
- Each command owns one file under src/commands/.
- Each command's documentation lives under docs/cli/commands/.

## Adding a command

1. Define the command contract and failure cases.
2. Add one implementation module under src/commands/.
3. Add command-specific host tests.
4. Add one documentation file under docs/cli/commands/.
5. Update COMMANDS.md, WORKFLOWS.md, and FILE_STRUCTURE.md when needed.
6. Update the roadmap before implementation if the capability changes scope.
7. Run the complete host and target validation set.

## Output and compatibility

Do not change output fields, error behavior, or exit codes casually. Update
OUTPUT.md, ERRORS.md, or EXIT_CODES.md with any intentional contract change.

## Dependency policy

Add a dependency only after checking its no_std relevance, license,
maintenance, auditability, transitive impact, and build cost. Host-only CLI
dependencies must not leak into embedded crates.
