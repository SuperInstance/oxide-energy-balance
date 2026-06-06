# oxide-energy-balance

Experiment: energy conservation laws as runtime invariants for GPU workloads. Proves ternary operations preserve algebraic invariants through the full compile→execute→verify cycle.

## Why This Matters

# oxide-energy-balance
Energy conservation laws as runtime invariants for GPU workloads.
Proves ternary operations preserve algebraic invariants through
the full compile → execute → verify cycle.

## The Five-Layer Stack

This crate is part of the **Oxide Stack** — a distributed GPU runtime built on five layers:

```
┌─────────────────┐
│  cudaclaw        │  Persistent GPU kernels, warp consensus, SmartCRDT
├─────────────────┤
│  cuda-oxide      │  Flux → MIR → Pliron → NVVM → PTX compiler
├─────────────────┤
│  flux-core       │  Bytecode VM + A2A agent protocol
├─────────────────┤
│  pincher         │  "Vector DB as runtime, LLM as compiler"
├─────────────────┤
│  open-parallel   │  Async runtime (tokio fork)
└─────────────────┘
```

The key insight: **ternary values {-1, 0, +1} map directly to GPU compute**. They pack 16× denser than FP32, enable XNOR+popcount matmul, and conservation laws become compile-time checks.

## Design

Every value in this crate follows **ternary algebra** (Z₃):

| Value | Meaning | GPU Analog |
|-------|---------|------------|
| +1 | Positive / Active / Healthy | Warp vote yes |
| 0 | Neutral / Pending / Balanced | Warp vote abstain |
| -1 | Negative / Failed / Overloaded | Warp vote no |

This isn't arbitrary — ternary is the natural encoding for:
1. **BitNet b1.58** (Microsoft) — ternary LLMs at 60% less power
2. **GPU warp voting** — hardware ballot returns ternary consensus
3. **Conservation laws** — {-1, 0, +1} preserves quantity

## Key Types

```rust
pub struct Energy
pub fn zero
pub fn from_trits
pub fn is_conserved
pub struct Workload
pub enum TernaryOp
pub struct EnergyVerifier
pub fn new
pub fn verify_op
pub fn verify_workload
pub fn checks_passed
pub fn checks_failed
```

## Usage

```toml
[dependencies]
oxide-energy-balance = "0.1.0"
```

```rust
use oxide_energy_balance::*;
// See src/lib.rs tests for complete working examples
```

## Testing

```bash
git clone https://github.com/SuperInstance/oxide-energy-balance.git
cd oxide-energy-balance
cargo test    # 7 tests
```

## Stats

| Metric | Value |
|--------|-------|
| Tests | 7 |
| Lines of Rust | 226 |
| Public API | 15 items |

## License

Apache-2.0
