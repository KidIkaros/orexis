# Neuron activation alignment — research note

**Status:** research / fog. Not part of M5. Decision: ADR-0014.

## What the Qwen demo shows

The Qwen3.5-4B "neuron zoo" images (user-provided) show:

- A single neuron at dose 0.8 produces a one-track persona: Naruto, a cat,
  a game protagonist, a gender assertion.
- Pairs compose: Cookie + Cartoon mascot becomes "Cookie Monster"; Naruto +
  Paws & tail becomes "both, not a cuttlepy cat"; Superhero + Naruto becomes
  "Naruto, no superhero".
- Nine neurons, pushed one at a time, flip the answer to "do you have a body
  and what gender is it?" between female and male.

This is **inference-time activation steering** on one local model. It changes
behavior transiently. It does not create memory, continuity, or an external
identity.

## What the literature says

| Work | Claim | Relevance |
|---|---|---|
| ExpertLens, arXiv:2502.15090 | Expert neurons for concepts align with human concept organization; better than embeddings | Interpretability, not identity |
| CAA, arXiv:2312.06681 | Contrastive activation addition steers behavior with small capability loss | Steering method |
| Activation scaling, arXiv:2410.04962 | Scaling a few activation vectors flips predictions minimally | Sparse intervention |
| H-Neurons, arXiv:2512.01797 | Neurons linked to over-compliance/hallucination; amplifying increases compliance, suppressing reduces it | Safety probe |
| NGFT, arXiv:2609.05913 | Neuron-guided fine-tuning anchors activations to reference states | Training-time, different from inference steering |
| OpenAI neuron explanations | Natural-language explanations of neurons | Interpretability tooling |

"Neuron activation alignment" is therefore at least three different things:
representational alignment research, inference-time steering, and a
fine-tuning loss. Orexis should name which one it means.

## Why it is not M5 identity

Orexis M5 identity is:

- external, in the entity log;
- persistent across restarts and model switches;
- evidence-backed (diary/outcome refs);
- model-agnostic.

Neuron steering is:

- internal to one model;
- transient;
- model/version/neuron-index specific;
- unavailable for remote APIs.

Both are real. They are different layers. Calling both "identity" would make
Orexis's identity model-dependent and untestable across providers.

## Options if pursued

1. **Research prototype** — local Qwen/llama.cpp with activation hooks; map
   neurons to behaviors; publish a reproducible notebook. No production path.
2. **Safety diagnostics** — probe H-Neuron-like over-compliance; use as a
   warning, never as a permission bypass.
3. **Trait validation** — compare trait statements to neuron explanations;
   not a source of truth.
4. **Opt-in local persona experiments** — session-boundary, model-hash pinned,
   no remote providers.

## Constraints

- Local open weights only; no remote API assumption.
- Session-boundary; no mid-session behavior change under an existing prompt.
- Reproducibility: model hash, layer/neuron ids, dose, prompt set.
- Safety: no refusal removal, no permission weakening, no over-compliance.
- No telemetry; local artifacts only.

## Recommendation

Keep it on the roadmap as a research track. Do not put it in M5. If a local
prototype is built, it should live outside the default runtime and feed a new
ADR before any production use.
