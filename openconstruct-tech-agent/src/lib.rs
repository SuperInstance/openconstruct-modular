#![deny(unsafe_code)]

use openconstruct_core::{Context, HealthStatus, Module};
use openconstruct_tech_math::MathEngine;
use openconstruct_tech_sensor::SensorEvent;

// ---------------------------------------------------------------------------
// AgentAction
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct AgentAction {
    pub agent_id: String,
    pub action_type: String,
    pub description: String,
    pub data: f64,
}

// ---------------------------------------------------------------------------
// SimpleAgent
// ---------------------------------------------------------------------------

pub const AGENT_ACTION_EVENT_TYPE: &str = "agent_action";

pub struct SimpleAgent {
    agent_id: String,
    started: bool,
    action_count: usize,
    last_action: Option<AgentAction>,
}

impl SimpleAgent {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            agent_id: id.into(),
            started: false,
            action_count: 0,
            last_action: None,
        }
    }

    /// Process sensor readings using the math engine from context.
    pub fn process_readings(&mut self, event: &SensorEvent, engine: &mut MathEngine) -> Vec<AgentAction> {
        let mut actions = Vec::new();
        let mut sum = 0.0;
        let mut count = 0usize;

        for reading in &event.readings {
            sum = engine.add(sum, reading.value);
            count += 1;
        }

        if count > 0 {
            let avg = engine.divide(sum, count as f64).unwrap_or(0.0);
            let action = AgentAction {
                agent_id: self.agent_id.clone(),
                action_type: "aggregate".into(),
                description: format!("Aggregated {} sensor readings", count),
                data: avg,
            };
            self.action_count += 1;
            self.last_action = Some(action.clone());
            actions.push(action);
        }

        actions
    }

    pub fn action_count(&self) -> usize {
        self.action_count
    }

    pub fn last_action(&self) -> Option<&AgentAction> {
        self.last_action.as_ref()
    }

    pub fn agent_id(&self) -> &str {
        &self.agent_id
    }
}

impl Module for SimpleAgent {
    fn name(&self) -> &str {
        "agent"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn start(&mut self, _ctx: &mut Context) {
        self.started = true;
    }

    fn stop(&mut self) {
        self.started = false;
    }

    fn health(&self) -> HealthStatus {
        if self.started {
            HealthStatus::Healthy
        } else {
            HealthStatus::Unhealthy("SimpleAgent not running".into())
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

    fn make_event(values: &[f64]) -> SensorEvent {
        let readings: Vec<SensorReading> = values
            .iter()
            .enumerate()
            .map(|(i, &v)| SensorReading {
                sensor_id: format!("s{}", i),
                value: v,
                unit: "x".into(),
                timestamp_ms: 100,
            })
            .collect();
        SensorEvent { readings }
    }

    #[test]
    fn agent_new() {
        let a = SimpleAgent::new("test-agent");
        assert_eq!(a.agent_id(), "test-agent");
        assert_eq!(a.action_count(), 0);
    }

    #[test]
    fn agent_process_readings() {
        let mut a = SimpleAgent::new("a1");
        let mut engine = MathEngine::new();
        let event = make_event(&[10.0, 20.0, 30.0]);
        let actions = a.process_readings(&event, &mut engine);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].data, 20.0); // average
    }

    #[test]
    fn agent_process_empty_event() {
        let mut a = SimpleAgent::new("a2");
        let mut engine = MathEngine::new();
        let event = SensorEvent { readings: vec![] };
        let actions = a.process_readings(&event, &mut engine);
        assert!(actions.is_empty());
    }

    #[test]
    fn agent_action_count_increments() {
        let mut a = SimpleAgent::new("a3");
        let mut engine = MathEngine::new();
        let e = make_event(&[1.0]);
        a.process_readings(&e, &mut engine);
        a.process_readings(&e, &mut engine);
        assert_eq!(a.action_count(), 2);
    }

    #[test]
    fn agent_last_action() {
        let mut a = SimpleAgent::new("a4");
        let mut engine = MathEngine::new();
        let e = make_event(&[5.0]);
        a.process_readings(&e, &mut engine);
        let last = a.last_action().unwrap();
        assert_eq!(last.data, 5.0);
    }

    #[test]
    fn agent_no_last_action_initially() {
        let a = SimpleAgent::new("a5");
        assert!(a.last_action().is_none());
    }

    #[test]
    fn agent_start_stop() {
        let mut a = SimpleAgent::new("a6");
        let mut ctx = Context::new();
        a.start(&mut ctx);
        assert!(a.started);
        a.stop();
        assert!(!a.started);
    }

    #[test]
    fn agent_health() {
        let mut a = SimpleAgent::new("a7");
        let mut ctx = Context::new();
        assert!(matches!(a.health(), HealthStatus::Unhealthy(_)));
        a.start(&mut ctx);
        assert_eq!(a.health(), HealthStatus::Healthy);
    }

    #[test]
    fn agent_name_version() {
        let a = SimpleAgent::new("x");
        assert_eq!(a.name(), "agent");
        assert_eq!(a.version(), "0.1.0");
    }

    #[test]
    fn agent_action_equality() {
        let a1 = AgentAction {
            agent_id: "a".into(),
            action_type: "t".into(),
            description: "d".into(),
            data: 1.0,
        };
        let a2 = a1.clone();
        assert_eq!(a1, a2);
    }

    #[test]
    fn agent_process_multiple_readings() {
        let mut a = SimpleAgent::new("multi");
        let mut engine = MathEngine::new();
        let event = make_event(&[2.0, 4.0, 6.0, 8.0]);
        let actions = a.process_readings(&event, &mut engine);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].data, 5.0); // (2+4+6+8)/4
    }

    #[test]
    fn agent_in_registry() {
        use openconstruct_core::ModuleRegistry;
        let mut reg = ModuleRegistry::new();
        reg.register(Box::new(SimpleAgent::new("r-agent")));
        let mut ctx = Context::new();
        reg.start_module("agent", &mut ctx);
        assert_eq!(reg.get("agent").unwrap().state, ModuleState::Running);
    }

    #[test]
    fn cross_module_integration() {
        // Start math and sensor, then agent processes sensor data via math engine
        let mut math = openconstruct_tech_math::MathModule::new();
        let mut sensor = openconstruct_tech_sensor::SensorModule::new();
        let mut agent = SimpleAgent::new("integ");

        let mut ctx = Context::new();
        math.start(&mut ctx);
        sensor.start(&mut ctx);
        agent.start(&mut ctx);

        let event = sensor.read_all();
        let engine = ctx.get_mut::<MathEngine>("math_engine").unwrap();
        let actions = agent.process_readings(&event, engine);
        assert_eq!(actions.len(), 1);
        assert!(actions[0].data > 0.0);
    }
}
