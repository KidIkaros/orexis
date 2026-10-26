# Origin hybrid signer (experimental)

An opt-in `Signer` adapter using `origin-crypto-sdk`'s Ed25519 + Falcon-1024
hybrid signature.

**Not a workspace member. Not built by default. Not production.**

The SDK is explicitly unaudited; see `docs/orexis/adr/0009-secrets-crypto.md`
and M6-R01. The default M6-01 evolution signer remains Ed25519
(`orexis-adapters-evolution-git::Ed25519Signer`).

```bash
cargo test --manifest-path experiments/origin-hybrid-signer/Cargo.toml
```
