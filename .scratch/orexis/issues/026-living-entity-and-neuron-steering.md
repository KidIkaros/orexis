# 026 — Living entity and neuron steering research (v0.5.0, M5 / R01)

- **Type:** research / feature
- **Blocks:** v0.5.0
- **Status:** open
- **Claimed by:** —

## Question

How should Orexis advance the living entity's lexical memory lattice in production, while running research track M5-R01 to evaluate local neuron activation steering without violating architectural boundaries (ADR-0014)?

## Evidence

- ADR-0013: Six drives (connection, curiosity, mastery, coherence, rest, growth) + PAD affect model.
- ADR-0014: Architectural boundary separating persistent, model-agnostic entity identity from transient, model-specific neuron activations.
- M5-R01 (`.scratch/orexis/m5/research/R01-neuron-alignment.md`): Local open-weights steering research.
- `docs/orexis/study/neuron-alignment.md`: Analysis of Qwen 2.5/3.5 neuron personas, `neuronpersonas.com`, `lyramakesmusic/neuron-explorer`, H-Neurons (arXiv:2512.01797), CAA (arXiv:2312.06681).
- Invariant 20: No vector RAG over code; lexical FTS5 + ripgrep + entity graph.

## Proposed Resolution

1. **Production Entity Memory Lattice**:
   - Complete FTS5 lexical recall and entity graph traversal in `crates/usecases/src/memory/`.
   - Maintain continuous self-model and diary reflections across 10+ sessions and 3+ provider switches without prompt leakage or context drift.
2. **Research Track M5-R01 Prototype**:
   - Build a standalone, reproducible local activation probe for Qwen/Llama open weights (via `llama.cpp` activation hooks or PyTorch runner).
   - Test H-Neuron suppression to reduce sycophancy and over-compliance in coding tasks.
   - Test cognitive posture steering (e.g. rigorous verification vs generative exploration).
   - Verify that neuron steering remains strictly opt-in, session-scoped, model-specific, and never conflated with the sovereign entity identity (ADR-0014).

## Follow-ups

- Publish reproducible notebook/script for M5-R01 on a research branch.
- Gate evaluation: Zero identity drift in `/self` across model switches; standalone evaluation report on sycophancy reduction.
