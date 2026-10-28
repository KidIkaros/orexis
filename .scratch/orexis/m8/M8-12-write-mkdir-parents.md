# M8-12 — write_file should create missing parent dirs

- **Status:** done — devin
- **Landed:** `resolve` canonicalizes the deepest *existing* ancestor and
  hangs the missing tail under it, so `write_file`'s `create_dir_all` is
  reachable. Red repro → regression tests in `files.rs`
  (`write_creates_missing_parents`, `write_still_rejects_escapes`).

## Bug

Bench surfaced it: `write-four-reports` runs die with
`resolve parent of reports/north.txt: No such file or directory` when the
model writes into a directory it hasn't created. The write tool should
mkdir parents, or the error should instruct the model to.

## Acceptance

- [x] Command that goes red on this bug, then a regression test.
- [x] write_file creates missing parents inside the sandbox root.
