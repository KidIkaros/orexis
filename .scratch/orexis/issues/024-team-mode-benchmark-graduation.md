# 024 — Team mode benchmark graduation (v0.3.0, M8 / §9)

- **Type:** task
- **Blocks:** v0.3.0
- **Status:** open
- **Claimed by:** —

## Question

What benchmark criteria and fixes are required to graduate Team Mode from experimental to GA across remote frontier and local models?

## Evidence

- `docs/orexis/PLAN.md` §9 & §10: Team mode gate requires $\ge 15\%$ higher pass rate or $\ge 25\%$ lower wall clock at $\le 2\times$ tokens compared to serial execution.
- `bench-results-qwen35-gate.jsonl`: Local Qwen3.5-2B paired run showed 5/5 pass for team vs 4/5 for serial, but mean wall clock was 18× higher due to worker stalling on approvals.
- M8-16 (`.scratch/orexis/m8/M8-16-listing-trap.md`): Context listing trap where workers loop on file listings.
- M8-17 (`.scratch/orexis/m8/M8-17-approval-resolution.md`): Unanswerable `board_add` approvals when workers dispatch tasks without user intervention.
- ADR-0020: Team-mode domain architecture (in-process turns sharing the entity).

## Proposed Resolution

1. **Resolve M8-16 and M8-17**:
   - Auto-delegate or propagate internal worker board mutations without hanging the client approval loop.
   - Enforce listing caps and tool output summarization inside team workers.
2. **Execute Frontier Benchmark Matrix**:
   - Run paired breadth benchmark across:
     - Local small (Qwen 3.5 2B / 4B)
     - Local medium (Llama 3.3 70B / Qwen 2.5 14B)
     - Remote frontier (Anthropic Claude 3.7 Sonnet, OpenAI o3/gpt-4.5)
     - Remote cheap (OpenAI gpt-4o-mini / DeepSeek V3)
3. **Provider Rate Limiting & Concurrency Control**:
   - Add backoff and token-bucket concurrency management in `orexis-adapters-provider-*`.
   - Prevent 429 rate limit cascades when 4+ workers run concurrently.

## Follow-ups

- If gate is cleared, flip team mode default availability in CLI (`orexis --team`).
- Document team mode strategies (`plan`, `build`, `review`, `mine`) in user docs.
