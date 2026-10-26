# 011 — Per-repo study decisions

- **Type:** research
- **Blocks:** synthesis
- **Status:** resolved
- **Claimed by:** devin, 2026-10-25

## Question

For each of the eight repositories, what is adopted, adapted, or rejected — with a citation per item — and which conflicts between them must be resolved in synthesis?

## Evidence

- `docs/orexis/study/_template.md`: twelve dimensions.
- `docs/orexis/study/synthesis.md`: pattern catalog, conflicts, ADRs to write.
- Plan §3: study order and method.

## Resolution

**Decided 2026-10-25.** All eight study documents are filled to adopt/adapt/reject; `docs/orexis/study/synthesis.md` is drafted with the pattern catalog, resolved conflicts, and the ADR map.

**Adopted from the internal repos**

- **Kindred:** five drives, glial modulation, pulse as a visible heartbeat, immune system, reflection tiers + metamorphic invariant, immutable soul corridor, orchestration modes/gates, planner→executor→verifier, MCP client, UTCP universal client, AEAD crypto + rusqlite.
- **Harness:** four codecs, pure `Effect` step, bounded shell, paired non-inferiority eval, genes seal, transport-agnostic daemon protocol, `Semaphore(1)` for local generation, remote wire profile.

**Adopted from the external repos**

- **DeepSeek Harness:** capability seams, three event domains, model-visible-means-logged, versioned immutable generations, fresh-process PTC, generated contracts, team limits.
- **opencode:** client/server, event-sourced sessions with branch/revert/fork, permission rulesets, syntax-aware command permissioning, model catalog + prompt matrix + cache-dialect fanout, code-mode as an opt-in surface, ACP server.
- **Hermes:** 43 provider profiles, prompt-caching invariant, policy floor under YOLO, untrusted-content delimiting, lineage compaction, skills + trust tiers, plugin compatibility as a behavior contract.
- **MUE-X:** six mutation strategies, immune safeguards, fitness, six-layer lattice, PAD affect.
- **Mercury:** installer + checksum + attach, daily budget + cheap-model routing, typed memory with confidence/conflicts, per-folder permissions, SSRF guards, verify-before-done, Ink fixes as a rendering reference.

**Rejected**

- Familiar's package sprawl, bootstrap-directory dependency, three identity files, breadth-first imitation.
- Kindred's companion product features and tone.
- Harness's sync `ToolExecutor` + `block_in_place` and single-model assumptions.
- DeepSeek's full unprivileged plugin core in v1.
- opencode's Effect/TS runtime and permissive defaults.
- MUE-X's always-on evolution loops and unapproved AST mutation.
- Mercury's TS runtime, cloud-first assumptions, and per-bot identity.

**Confidence:** high for Kindred, Harness, Hermes (local source read); medium for DeepSeek, opencode, MUE-X, Mercury (corpus study + docs); Familiar is a failure analysis from its own design documentation.

**Remaining verification:** DeepSeek/Hermes migration semantics, MUE-X fitness computation, Mercury budget accounting. These are marked *(secondary)* in the synthesis and re-verified during the port.

## Follow-ups

- The study gate still requires the S4 vertical slice before M1.
- Ticket 007 consumes the drives question; ticket 008 consumes the evolution details; ticket 009 consumes the native-adapter evidence.
- The synthesis's ADR map is the input to the M1 architecture ADRs.
