# Familiar — study (failure analysis)

**Source:** `github.com/KidIkaros/familiar` (private) · **Language:** Go · **Size:** 158k LOC, 73 packages · **License:** private · **Last seen:** via GitHub

**Why it is in the study:** it is the cautionary tale, and its own design documentation says so. Familiar is what Orexis must not become.

## The admission

Kindred's design document names the failure directly:

> "familiar v1 tried to be a better chatbot. It accumulated 73 packages, 158K lines of Go, three identity files, a daemon that couldn't start without a bootstrap directory… The ideas were brilliant — drives, reflection, autonomy — but they were buried under infrastructure built to imitate competitors."

That is the same shape Orexis would take if it chased Hermes's surface area: breadth first, architecture second.

## Dimensions to fill

1. **Process model** — daemon + bootstrap directory. Why did startup depend on a directory?
2. **Turn loop** — was there one, or several?
3. **Session model** — what was persisted, and what was lost on restart?
4. **Tool protocol** — how many tools, and how were they described?
5. **Context management** — did context assembly exist, or was it ad hoc?
6. **Provider abstraction** — how many providers, and at what maintenance cost?
7. **Entity substrate** — three identity files. Which was authoritative?
8. **Evolution** — what was attempted, and what broke?
9. **Interop** — IPC and event bus. What did they actually buy?
10. **Surfaces** — how many, and which owned state?
11. **Quality practice** — what tests existed, and what did they miss?
12. **Failure modes** — the load-bearing section.

## Known evidence

- 73 packages, 158k LOC of Go — more than Kindred and Harness combined.
- Three identity files with no single source of truth.
- A daemon that could not start without a bootstrap directory.
- Infrastructure built to imitate competitors rather than to serve the entity.

## Adopt

- Event bus and IPC *ideas* — only if they earn a seam.
- Skills and curator *ideas* — only where Kindred does not already have them.

## Adapt

- <to fill>

## Reject

- Package sprawl as a proxy for capability.
- Bootstrap-directory startup dependency.
- Multiple identity files.
- Breadth-first imitation of competitor surfaces.

## Open questions

- Which specific modules created the bootstrap dependency?
- What did the event bus do well, and where did it become a second source of truth?
- Which identity file won when they disagreed?
