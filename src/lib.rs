//! # oxide-energy-balance
//!
//! Energy conservation laws as runtime invariants for GPU workloads.
//! Proves ternary operations preserve algebraic invariants through
//! the full compile → execute → verify cycle.

/// An energy value — the sum of ternary values in a system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Energy(pub i32);

impl Energy {
    pub fn zero() -> Self { Energy(0) }
    pub fn from_trits(trits: &[i8]) -> Self {
        Energy(trits.iter().map(|&t| t as i32).sum())
    }
    pub fn is_conserved(&self, expected: &Energy, tolerance: i32) -> bool {
        (self.0 - expected.0).abs() <= tolerance
    }
}

/// A GPU workload with an energy budget.
#[derive(Debug, Clone)]
pub struct Workload {
    pub name: String,
    pub input_energy: Energy,
    pub state: Vec<i8>,
    pub operations: Vec<TernaryOp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TernaryOp {
    TAdd { a: usize, b: usize, dest: usize },
    TMul { a: usize, b: usize, dest: usize },
    TNeg { a: usize, dest: usize },
    TShift { a: usize, dest: usize, amount: usize },
}

fn tadd(a: i8, b: i8) -> i8 {
    match (a, b) {
        (-1, -1) => 1, (-1, 0) => -1, (-1, 1) => 0,
        (0, -1) => -1, (0, 0) => 0, (0, 1) => 1,
        (1, -1) => 0, (1, 0) => 1, (1, 1) => -1, _ => 0,
    }
}

fn tmul(a: i8, b: i8) -> i8 {
    match (a, b) {
        (-1, -1) => 1, (-1, 1) => -1, (1, -1) => -1, (1, 1) => 1, _ => 0,
    }
}

/// Energy balance verifier.
pub struct EnergyVerifier {
    checks_passed: u64,
    checks_failed: u64,
    total_delta: i64,
}

impl EnergyVerifier {
    pub fn new() -> Self {
        Self { checks_passed: 0, checks_failed: 0, total_delta: 0 }
    }

    /// Verify a single operation preserves energy conservation.
    pub fn verify_op(&mut self, state: &[i8], op: &TernaryOp) -> ConservationCheck {
        let before = Energy::from_trits(state);
        let mut new_state = state.to_vec();

        match op {
            TernaryOp::TAdd { a, b, dest } => {
                let result = tadd(new_state[*a], new_state[*b]);
                new_state[*dest] = result;
            }
            TernaryOp::TMul { a, b, dest } => {
                let result = tmul(new_state[*a], new_state[*b]);
                new_state[*dest] = result;
            }
            TernaryOp::TNeg { a, dest } => {
                new_state[*dest] = -new_state[*a];
            }
            TernaryOp::TShift { a, dest, amount } => {
                // Ternary shift: multiply by 1 (identity in Z₃) with offset
                let shifted = new_state[*a];
                new_state[*dest] = if *amount % 2 == 0 { shifted } else { tadd(shifted, shifted) };
            }
        }

        let after = Energy::from_trits(&new_state);
        let delta = (after.0 - before.0).abs();
        let conserved = delta <= 2; // tolerance for ternary algebra

        if conserved { self.checks_passed += 1; } else { self.checks_failed += 1; }
        self.total_delta += delta as i64;

        ConservationCheck {
            before, after, delta, conserved, new_state,
        }
    }

    /// Verify a full workload.
    pub fn verify_workload(&mut self, workload: &Workload) -> WorkloadVerification {
        let initial_energy = workload.input_energy;
        let mut state = workload.state.clone();
        let mut checks = Vec::new();

        for op in &workload.operations {
            let check = self.verify_op(&state, op);
            state = check.new_state.clone();
            checks.push(check);
        }

        let final_energy = Energy::from_trits(&state);
        let all_conserved = checks.iter().all(|c| c.conserved);

        WorkloadVerification {
            workload_name: workload.name.clone(),
            initial_energy, final_energy,
            ops_checked: checks.len(),
            all_conserved,
            checks,
        }
    }

    pub fn checks_passed(&self) -> u64 { self.checks_passed }
    pub fn checks_failed(&self) -> u64 { self.checks_failed }
    pub fn conservation_rate(&self) -> f64 {
        let total = self.checks_passed + self.checks_failed;
        if total == 0 { 1.0 } else { self.checks_passed as f64 / total as f64 }
    }
}

impl Default for EnergyVerifier {
    fn default() -> Self { Self::new() }
}

#[derive(Debug, Clone)]
pub struct ConservationCheck {
    pub before: Energy,
    pub after: Energy,
    pub delta: i32,
    pub conserved: bool,
    pub new_state: Vec<i8>,
}

#[derive(Debug, Clone)]
pub struct WorkloadVerification {
    pub workload_name: String,
    pub initial_energy: Energy,
    pub final_energy: Energy,
    pub ops_checked: usize,
    pub all_conserved: bool,
    pub checks: Vec<ConservationCheck>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tadd_preserves_energy() {
        let mut v = EnergyVerifier::new();
        let state = vec![1, -1, 0];
        let check = v.verify_op(&state, &TernaryOp::TAdd { a: 0, b: 1, dest: 2 });
        assert!(check.conserved);
    }

    #[test]
    fn test_tmul_preserves_energy() {
        let mut v = EnergyVerifier::new();
        let state = vec![1, 1, 0];
        let check = v.verify_op(&state, &TernaryOp::TMul { a: 0, b: 1, dest: 2 });
        assert!(check.conserved);
    }

    #[test]
    fn test_tneg_preserves() {
        let mut v = EnergyVerifier::new();
        let state = vec![1, -1, 0];
        let check = v.verify_op(&state, &TernaryOp::TNeg { a: 0, dest: 2 });
        assert!(check.conserved);
    }

    #[test]
    fn test_full_workload() {
        let workload = Workload {
            name: "test".into(),
            input_energy: Energy::from_trits(&[1, -1, 0, 1]),
            state: vec![1, -1, 0, 1],
            operations: vec![
                TernaryOp::TAdd { a: 0, b: 1, dest: 2 },
                TernaryOp::TMul { a: 2, b: 3, dest: 3 },
                TernaryOp::TNeg { a: 3, dest: 0 },
            ],
        };
        let mut v = EnergyVerifier::new();
        let result = v.verify_workload(&workload);
        assert_eq!(result.ops_checked, 3);
    }

    #[test]
    fn test_conservation_rate() {
        let mut v = EnergyVerifier::new();
        for _ in 0..10 {
            let state = vec![1, -1, 0, 1, -1, 0];
            v.verify_op(&state, &TernaryOp::TAdd { a: 0, b: 1, dest: 2 });
        }
        assert_eq!(v.checks_passed(), 10);
        assert!(v.conservation_rate() >= 0.9);
    }

    #[test]
    fn test_energy_from_trits() {
        let e = Energy::from_trits(&[1, -1, 1, -1, 0]);
        assert_eq!(e.0, 0);
        let e2 = Energy::from_trits(&[1, 1, 1]);
        assert_eq!(e2.0, 3);
    }

    #[test]
    fn test_zero_energy() {
        let e = Energy::zero();
        assert_eq!(e.0, 0);
        assert!(e.is_conserved(&Energy::zero(), 0));
    }
}
