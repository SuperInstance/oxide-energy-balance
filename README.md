# oxide-energy-balance

Energy conservation laws as runtime invariants for ternary GPU workloads.

## Why This Exists

Computational correctness isn't just about getting the right output — it's about preserving structural invariants through every operation. For ternary arithmetic (values in {-1, 0, +1} with Z₃ algebra), the "energy" of a system is the sum of its trit values. If that sum changes unexpectedly after an operation, something went wrong: a bit flip, a memory corruption, a compiler bug.

This crate implements energy conservation verification for every ternary operation in a GPU workload. Each operation (TAdd, TMul, TNeg, TShift) is checked individually, and the full workload is verified end-to-end. The tolerance is configurable (default: ±2 per operation) because ternary algebra doesn't perfectly conserve the sum in every case, but bounded deviation from conservation is a strong signal of correctness.

## Architecture

```
┌──────────────────────────────────────────────┐
│            EnergyVerifier                     │
│  checks_passed: 47                           │
│  checks_failed: 0                            │
│  conservation_rate: 1.0 (100%)               │
│                                              │
│  verify_op(state, op) → ConservationCheck    │
│  ┌────────────────────────────────────────┐  │
│  │ Before: Energy(0)  [1, -1, 0]         │  │
│  │ Op: TAdd { a:0, b:1, dest:2 }         │  │
│  │   tadd(1, -1) = 0 → state[2] = 0     │  │
│  │ After:  Energy(0)  [1, -1, 0]         │  │
│  │ Delta: 0  ✓ Conserved                 │  │
│  └────────────────────────────────────────┘  │
│                                              │
│  verify_workload(workload) → Verification    │
│  ┌────────────────────────────────────────┐  │
│  │ "reduce_pipeline"                      │  │
│  │ Initial energy: 3                      │  │
│  │ ┌─ TAdd r0,r1→r2  delta=0 ✓         │  │
│  │ ├─ TMul r2,r3→r3  delta=1 ✓         │  │
│  │ └─ TNeg r3→r0     delta=0 ✓         │  │
│  │ Final energy: 2                        │  │
│  │ All conserved: true                    │  │
│  └────────────────────────────────────────┘  │
│                                              │
│  Z₃ Arithmetic:                              │
│  TAdd: modular addition in {-1, 0, +1}       │
│    (-1)+(-1)=+1, (-1)+0=-1, 0+0=0, 1+1=-1  │
│  TMul: modular multiplication                │
│    (-1)×(-1)=+1, (-1)×(+1)=-1, ×0=0        │
│  TNeg: negation                              │
│    -(-1)=+1, -(0)=0, -(+1)=-1              │
│  TShift: even=identity, odd=doubling(TAdd)   │
└──────────────────────────────────────────────┘
```

**Key types:**

- `Energy` — wrapper around i32, sum of ternary values
- `TernaryOp` — `TAdd`, `TMul`, `TNeg`, `TShift` with source/dest indices
- `Workload` — named workload with input energy, initial state, and operation sequence
- `ConservationCheck` — per-operation result: before/after energy, delta, conserved flag
- `WorkloadVerification` — full workload result: initial/final energy, all checks, overall status
- `EnergyVerifier` — the verification engine

## Usage

```rust
use oxide_energy_balance::*;

// Verify a single operation
let mut verifier = EnergyVerifier::new();
let state = vec![1, -1, 0];
let check = verifier.verify_op(&state, &TernaryOp::TAdd { a: 0, b: 1, dest: 2 });
assert!(check.conserved);
assert_eq!(check.delta, 0);

// Verify a full workload
let workload = Workload {
    name: "reduce_pipeline",
    input_energy: Energy::from_trits(&[1, -1, 0, 1]),
    state: vec![1, -1, 0, 1],
    operations: vec![
        TernaryOp::TAdd { a: 0, b: 1, dest: 2 },
        TernaryOp::TMul { a: 2, b: 3, dest: 3 },
        TernaryOp::TNeg { a: 3, dest: 0 },
    ],
};
let result = verifier.verify_workload(&workload);
assert!(result.all_conserved);
println!("Ops checked: {}", result.ops_checked);
println!("Initial energy: {}", result.initial_energy.0);
println!("Final energy: {}", result.final_energy.0);

// Energy from trits
let e = Energy::from_trits(&[1, -1, 1, -1, 0]);
assert_eq!(e.0, 0); // balanced

// Conservation check with tolerance
let e1 = Energy(5);
let e2 = Energy(7);
assert!(e1.is_conserved(&e2, 3)); // |5-7| = 2 ≤ 3
```

## API Reference

### `Energy`

```rust
pub struct Energy(pub i32);
```

- `zero() -> Self` — zero energy
- `from_trits(trits: &[i8]) -> Self` — sum of ternary values
- `is_conserved(&self, expected: &Energy, tolerance: i32) -> bool` — |self - expected| ≤ tolerance

### `TernaryOp`

```rust
pub enum TernaryOp {
    TAdd { a: usize, b: usize, dest: usize },
    TMul { a: usize, b: usize, dest: usize },
    TNeg { a: usize, dest: usize },
    TShift { a: usize, dest: usize, amount: usize },
}
```

### `Workload`

```rust
pub struct Workload {
    pub name: String,
    pub input_energy: Energy,
    pub state: Vec<i8>,
    pub operations: Vec<TernaryOp>,
}
```

### `ConservationCheck`

```rust
pub struct ConservationCheck {
    pub before: Energy,
    pub after: Energy,
    pub delta: i32,
    pub conserved: bool,
    pub new_state: Vec<i8>,
}
```

### `WorkloadVerification`

```rust
pub struct WorkloadVerification {
    pub workload_name: String,
    pub initial_energy: Energy,
    pub final_energy: Energy,
    pub ops_checked: usize,
    pub all_conserved: bool,
    pub checks: Vec<ConservationCheck>,
}
```

### `EnergyVerifier`

- `new() -> Self`
- `verify_op(state: &[i8], op: &TernaryOp) -> ConservationCheck` — single operation verification
- `verify_workload(workload: &Workload) -> WorkloadVerification` — full workload verification
- `checks_passed() -> u64` / `checks_failed() -> u64`
- `conservation_rate() -> f64` — passed / total

## The Deeper Idea

This is the **correctness layer** in the oxide stack's verification architecture. Every other crate in the ecosystem operates on ternary values {-1, 0, +1} and assumes that the algebra is sound. This crate proves it — at runtime, for every operation, on every execution.

The connection to oxide-pipeline is direct: the pipeline's Flux VM executes exactly these TAdd/TMul/TNeg operations on exactly this ternary data. The pipeline's conservation check (`verify_conservation`) is a coarse-grained version of what this crate does precisely. The pipeline checks that the sum of inputs is approximately equal to the sum of outputs; this crate checks that every individual operation preserves energy within tolerance.

The Z₃ algebra is not arbitrary. It's the ring of integers modulo 3, which means every element has a multiplicative inverse and addition is always invertible. This is why energy conservation is even possible — the algebra is closed and well-behaved. The tolerance of ±2 accounts for the fact that while individual operations are well-behaved, composition can cause the sum to drift slightly due to the specific mapping from Z₃ to {-1, 0, +1} representation.

## Related Crates

- **oxide-pipeline** — executes ternary operations that this crate verifies
- **oxide-checkpoint** — saves verified state for recovery with energy guarantees
- **oxide-journal** — logs every operation, enabling post-hoc energy auditing
- **oxide-gradient** — optimization that must preserve energy conservation in tuned kernels
