# ADR-0014 — Entity identity is not model persona

- **Status:** accepted
- **Date:** 2026-10-26
- **Deciders:** project owner, Devin
- **Related:** ADR-0013, M5, issue 012

## Context

The Qwen3.5-4B "neuron zoo" demonstration shows that nudging a single neuron
(or a pair) at inference time produces a one-track persona: Naruto, a cat,
a game protagonist, a gender assertion. Pairs compose. The behavior is
striking, but it is **transient, model-specific, and induced by activation
steering** — not a persistent identity.

The literature distinguishes three things that are often conflated:

- **Representational alignment** — expert neurons for concepts align with
  human concept organization (ExpertLens, arXiv:2502.15090).
- **Activation steering** — inference-time interventions on neurons or
  residual streams change behavior (CAA, arXiv:2312.06681; activation
  scaling, arXiv:2410.04962).
- **Neuron-guided fine-tuning (NGFT)** — a training loss anchors activations
  to reference states (arXiv:2609.05913).
- **Safety probes** — H-Neurons are associated with over-compliance and
  hallucination; amplifying them increases compliance, suppressing them
  reduces it (arXiv:2512.01797).

Orexis M5 defines entity identity as external, persistent, evidence-backed
traits, drives, affect, and memory (ADR-0013). That is a different layer from
the model's internal persona.

## Decision

1. **Entity identity is external and model-agnostic.** It lives in the entity
   log, survives restarts and model switches, and is evidence-backed.
2. **Model persona is internal and model-specific.** It is induced by weights,
   activations, or steering in one model. It is not the entity and does not
   persist across providers.
3. **Neuron activation alignment/steering is a research track, not M5.**
   It does not enter the M5 implementation or the default runtime.
4. **If pursued, it is local-only and opt-in.** It requires open weights and
   activation access; remote APIs are out of scope. It must record the model
   hash, layer/neuron indices, dose, and prompt set for reproducibility.
5. **Session-boundary only.** No mid-session steering that changes behavior
   under an existing prompt; the prompt-cache and identity-leak invariants
   still hold.
6. **Safety floor is absolute.** Steering may not remove refusals, weaken
   permissions, bypass plan mode, or be used for over-compliance. Safety
   probes are diagnostics, not a production control.
7. **Not a source of truth for traits.** Neuron explanations may inform
   validation or research, but a trait still requires diary/outcome evidence.

## Consequences

- M5 stays model-agnostic and portable; no provider is required to expose
  neurons.
- A future research prototype can run against a local Qwen/llama.cpp model
  without changing the entity contract.
- The "identity" word is no longer overloaded: entity identity vs model
  persona is explicit in `CONTEXT.md`.
- Any steering feature needs its own ADR, safety review, and evaluation.

## Alternatives considered

- **Integrate steering into M5** — rejected: model-specific, no activation
  access for remote providers, and it would conflate persona with identity.
- **Ignore the research** — rejected: it is relevant to safety and to
  understanding model behavior; it belongs on the roadmap.
- **Treat neuron explanations as trait evidence** — rejected: it would make
  identity model-dependent and unverifiable across providers.
