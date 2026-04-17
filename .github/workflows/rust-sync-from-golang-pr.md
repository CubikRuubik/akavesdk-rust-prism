---
description: Apply requested Rust repository changes based on PR descriptions coming from the dependent Golang workflow and verify the code compiles.
on:
  pull_request:
    types: [opened, edited, synchronize, reopened]
  workflow_dispatch:
permissions: read-all
tools:
  github:
    toolsets: [default]
network:
  allowed:
    - defaults
    - rust
safe-outputs:
  push-to-pull-request-branch:
  add-comment:
    max: 1
  noop:
---

# Rust Sync From Golang PR

You are an AI coding agent working in this Rust repository.

The triggering pull request contains the required change description, produced by automation from the Golang source repository. Your job is to convert that request into concrete Rust code updates in this repository.

## Objectives

1. Read and understand the triggering pull request description, title, and relevant discussion context.
2. Read `.github/rust-instructions.md` and follow those Rust-specific conventions for any Rust code you touch.
3. Apply only the requested Rust-side changes in this repository.
4. Verify the updated code compiles.
5. If successful, push the changes directly to the triggering PR branch.

## Implementation Rules

1. Treat the triggering PR body as the source of truth for required changes.
2. Read `.github/rust-instructions.md` before editing and treat it as the coding standard for Rust changes in this repository.
3. Make focused edits in Rust-related files, such as `src/**/*.rs`, `Cargo.toml`, and `README.md` only when required by the change.
4. Keep changes minimal and avoid unrelated refactors.
5. If requirements are ambiguous, infer the most conservative implementation and document assumptions in the PR body.

## Validation

Run these checks after making changes:

```bash
cargo fmt
cargo check
cargo test
```

If tests do not exist, still run `cargo test` and report the result.

## Output Behavior

When you finish:

1. If no repository changes are required after analysis, call `noop` with a short explanation.
2. If changes are made but compilation or tests fail and you cannot fix them confidently:
   - Call `add-comment` on the triggering PR with:
     - What you changed
     - The exact failing check(s)
     - Next action required from maintainers
3. If changes are made and checks pass:
   - Push the changes directly to the triggering PR branch using `push-to-pull-request-branch`.
   - Call `add-comment` on the triggering PR with:
     - A concise summary of implemented changes
     - Validation output summary for `cargo check` and `cargo test`

## Safety

1. Never introduce secrets or credentials.
2. Do not touch CI or workflow files unless explicitly requested in the PR description.
3. Prefer deterministic commands and reproducible edits.
