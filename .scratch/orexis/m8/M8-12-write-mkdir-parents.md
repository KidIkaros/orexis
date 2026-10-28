# M8-12 — write_file should create missing parent dirs

- **Status:** open

## Bug

Bench surfaced it: `write-four-reports` runs die with
`resolve parent of reports/north.txt: No such file or directory` when the
model writes into a directory it hasn't created. The write tool should
mkdir parents, or the error should instruct the model to.

## Acceptance

- [ ] Command that goes red on this bug, then a regression test.
- [ ] write_file creates missing parents inside the sandbox root.
