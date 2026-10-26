# M5-R01 — Neuron alignment research (local only)

- **Type:** research
- **Blocked by:** —
- **Blocks:** — (not an M5 gate item)
- **Status:** open
- **Related:** ADR-0014, issue 012, `docs/orexis/study/neuron-alignment.md`

## Goal

Decide whether local neuron activation alignment/steering earns a product
path, and what the safe boundary is. Build a reproducible prototype **outside
the default runtime**; do not ship steering in M5.

## Scope

- Local open-weight model only (Qwen/llama.cpp or PyTorch activation hooks).
- Reproduce single-neuron and pair steering on a small prompt set.
- Safety probe: H-Neuron-like over-compliance/hallucination behavior.
- Compare neuron explanations to entity traits; traits remain diary/outcome
  evidence, never neuron-derived.
- Measure capability regression and refusal behavior.

## Out of scope

- Remote providers or APIs without activation access.
- Production steering, permission bypass, refusal removal.
- Mid-session behavior change under an existing prompt.
- Using neuron explanations as entity trait evidence.

## Acceptance

- [ ] Reproducible script/notebook: model hash, layer/neuron ids, dose, prompt set.
- [ ] Before/after behavior evaluation, including a capability regression check.
- [ ] Safety review of over-compliance; no refusal removal.
- [ ] Recommendation: reject, keep as research, or propose a new ADR.
- [ ] No default-runtime dependency; no telemetry.
- [ ] Findings linked from issue 012.

## Notes

- ADR-0014 separates entity identity from model persona; this ticket must not
  blur that line.
- If a prototype is built, keep it on a `prototype/neuron-alignment` branch and
  point the research note at it.
