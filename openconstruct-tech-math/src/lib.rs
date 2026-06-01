#![deny(unsafe_code)]

use openconstruct_core::{Context, HealthStatus, Module};

// ---------------------------------------------------------------------------
// MathEngine
// ---------------------------------------------------------------------------

/// Shared math engine that gets placed in the Context for other modules.
pub struct MathEngine {
    ops_run: usize,
}

impl MathEngine {
    pub fn new() -> Self {
        Self { ops_run: 0 }
    }

    pub fn add(&mut self, a: f64, b: f64) -> f64 {
        self.ops_run += 1;
        a + b
    }

    pub fn subtract(&mut self, a: f64, b: f64) -> f64 {
        self.ops_run += 1;
        a - b
    }

    pub fn multiply(&mut self, a: f64, b: f64) -> f64 {
        self.ops_run += 1;
        a * b
    }

    pub fn divide(&mut self, a: f64, b: f64) -> Option<f64> {
        if b == 0.0 {
            return None;
        }
        self.ops_run += 1;
        Some(a / b)
    }

    /// Transpose a 2-D matrix (vector of vectors).
    pub fn transpose(&self, m: &[Vec<f64>]) -> Vec<Vec<f64>> {
        if m.is_empty() {
            return vec![];
        }
        let rows = m.len();
        let cols = m[0].len();
        let mut out = vec![vec![0.0; rows]; cols];
        for (r, row) in m.iter().enumerate() {
            for (c, &val) in row.iter().enumerate() {
                out[c][r] = val;
            }
        }
        out
    }

    /// Element-wise matrix addition.
    pub fn mat_add(&self, a: &[Vec<f64>], b: &[Vec<f64>]) -> Option<Vec<Vec<f64>>> {
        if a.len() != b.len() {
            return None;
        }
        let mut out = Vec::with_capacity(a.len());
        for (ra, rb) in a.iter().zip(b.iter()) {
            if ra.len() != rb.len() {
                return None;
            }
            out.push(ra.iter().zip(rb.iter()).map(|(x, y)| x + y).collect());
        }
        Some(out)
    }

    /// Dot product of two vectors.
    pub fn dot(&self, a: &[f64], b: &[f64]) -> Option<f64> {
        if a.len() != b.len() {
            return None;
        }
        Some(a.iter().zip(b.iter()).map(|(x, y)| x * y).sum())
    }

    pub fn ops_run(&self) -> usize {
        self.ops_run
    }
}

impl Default for MathEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// MathModule
// ---------------------------------------------------------------------------

const CTX_KEY: &str = "math_engine";

pub struct MathModule {
    started: bool,
}

impl MathModule {
    pub fn new() -> Self {
        Self { started: false }
    }
}

impl Default for MathModule {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for MathModule {
    fn name(&self) -> &str {
        "math"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn start(&mut self, ctx: &mut Context) {
        ctx.set(CTX_KEY, MathEngine::new());
        self.started = true;
    }

    fn stop(&mut self) {
        self.started = false;
    }

    fn health(&self) -> HealthStatus {
        if self.started {
            HealthStatus::Healthy
        } else {
            HealthStatus::Unhealthy("MathModule is not running".into())
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use openconstruct_core::ModuleState;

    #[test]
    fn math_engine_add() {
        let mut e = MathEngine::new();
        assert_eq!(e.add(2.0, 3.0), 5.0);
    }

    #[test]
    fn math_engine_subtract() {
        let mut e = MathEngine::new();
        assert_eq!(e.subtract(10.0, 4.0), 6.0);
    }

    #[test]
    fn math_engine_multiply() {
        let mut e = MathEngine::new();
        assert_eq!(e.multiply(3.0, 7.0), 21.0);
    }

    #[test]
    fn math_engine_divide() {
        let mut e = MathEngine::new();
        assert_eq!(e.divide(10.0, 2.0), Some(5.0));
    }

    #[test]
    fn math_engine_divide_by_zero() {
        let mut e = MathEngine::new();
        assert_eq!(e.divide(1.0, 0.0), None);
    }

    #[test]
    fn math_engine_transpose() {
        let m = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
        let t = MathEngine::new().transpose(&m);
        assert_eq!(t, vec![vec![1.0, 3.0], vec![2.0, 4.0]]);
    }

    #[test]
    fn math_engine_transpose_empty() {
        let t = MathEngine::new().transpose(&[]);
        assert!(t.is_empty());
    }

    #[test]
    fn math_engine_mat_add() {
        let a = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
        let b = vec![vec![5.0, 6.0], vec![7.0, 8.0]];
        let r = MathEngine::new().mat_add(&a, &b);
        assert_eq!(r, Some(vec![vec![6.0, 8.0], vec![10.0, 12.0]]));
    }

    #[test]
    fn math_engine_mat_add_mismatch_rows() {
        let a = vec![vec![1.0]];
        let b = vec![vec![1.0], vec![2.0]];
        assert!(MathEngine::new().mat_add(&a, &b).is_none());
    }

    #[test]
    fn math_engine_mat_add_mismatch_cols() {
        let a = vec![vec![1.0, 2.0]];
        let b = vec![vec![1.0]];
        assert!(MathEngine::new().mat_add(&a, &b).is_none());
    }

    #[test]
    fn math_engine_dot() {
        let r = MathEngine::new().dot(&[1.0, 2.0, 3.0], &[4.0, 5.0, 6.0]);
        assert_eq!(r, Some(32.0));
    }

    #[test]
    fn math_engine_dot_mismatch() {
        let r = MathEngine::new().dot(&[1.0], &[1.0, 2.0]);
        assert!(r.is_none());
    }

    #[test]
    fn math_engine_ops_count() {
        let mut e = MathEngine::new();
        e.add(1.0, 1.0);
        e.multiply(2.0, 3.0);
        assert_eq!(e.ops_run(), 2);
    }

    #[test]
    fn math_module_start_registers_engine() {
        let mut m = MathModule::new();
        let mut ctx = Context::new();
        m.start(&mut ctx);
        assert!(ctx.get::<MathEngine>(CTX_KEY).is_some());
    }

    #[test]
    fn math_module_health_running() {
        let mut m = MathModule::new();
        let mut ctx = Context::new();
        m.start(&mut ctx);
        assert_eq!(m.health(), HealthStatus::Healthy);
    }

    #[test]
    fn math_module_health_stopped() {
        let mut m = MathModule::new();
        let mut ctx = Context::new();
        m.start(&mut ctx);
        m.stop();
        match m.health() {
            HealthStatus::Unhealthy(_) => {}
            other => panic!("expected Unhealthy, got {:?}", other),
        }
    }

    #[test]
    fn math_module_name_version() {
        let m = MathModule::new();
        assert_eq!(m.name(), "math");
        assert_eq!(m.version(), "0.1.0");
    }

    #[test]
    fn math_module_default() {
        let m = MathModule::default();
        assert_eq!(m.name(), "math");
    }

    #[test]
    fn math_module_in_registry() {
        use openconstruct_core::ModuleRegistry;
        let mut reg = ModuleRegistry::new();
        reg.register(Box::new(MathModule::new()));
        let mut ctx = Context::new();
        reg.start_module("math", &mut ctx);
        assert_eq!(reg.get("math").unwrap().state, ModuleState::Running);
        assert!(ctx.get::<MathEngine>(CTX_KEY).is_some());
    }
}
