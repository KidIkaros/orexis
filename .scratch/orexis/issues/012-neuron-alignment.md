# 012 — Neuron activation alignment and steering

- **Type:** research
- **Blocks:** —
- **Status:** resolved as a research track (ADR-0014); M5-R01 remains open
- **Claimed by:** —

## Question

Should Orexis support local neuron activation alignment/steering for persona
or safety research, and what is the safe boundary?

## Evidence

- Qwen3.5-4B "neuron zoo" demo (user-provided): single-neuron personas, pairs,
  gender neurons; inference-time steering.
- ExpertLens, arXiv:2502.15090 — expert neurons align with human concepts.
- CAA, arXiv:2312.06681 — contrastive activation addition.
- Activation scaling, arXiv:2410.04962 — sparse activation intervention.
- H-Neurons, arXiv:2512.01797 — over-compliance/hallucination neurons.
- NGFT, arXiv:2609.05913 — neuron-guided fine-tuning.
- OpenAI neuron explanations.

## Resolution

Not M5. ADR-0014: entity identity is external/persistent/model-agnostic;
model persona is internal/model-specific. Neuron alignment/steering is a
research track: local open weights only, opt-in, session-boundary,
reproducible, safety-reviewed, never a permission bypass.

## Follow-ups

- M5-R01 (`.scratch/orexis/m5/research/R01-neuron-alignment.md`).
- Local prototype (Qwen/llama.cpp activation access) outside the default runtime.
- Safety-probe evaluation for over-compliance.
- A new ADR before any production steering feature.
