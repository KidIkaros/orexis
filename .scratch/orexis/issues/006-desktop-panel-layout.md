# 006 — Desktop panel layout in Leptos

- **Type:** prototype
- **Blocks:** M2
- **Status:** open
- **Claimed by:** —

## Question

What is the Leptos layout and state model for the six panels — chat/streaming, fleet, approvals, diff viewer, artifacts, file browser — over the shared daemon socket?

## Evidence

- Plan §8.3: six panels; settings for model/provider, persona, permissions, privacy.
- Plan §8.1: desktop attaches over the same Unix socket through its Rust side; no HTTP server.
- opencode: client/server with generated contracts.
- Hermes desktop: Bot Mode, slash palette.

## Resolution

<filled when resolved>

## Follow-ups

<new tickets or fog this resolution creates>
