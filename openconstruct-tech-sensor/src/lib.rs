#![deny(unsafe_code)]

use openconstruct_core::{Context, HealthStatus, Module};

// ---------------------------------------------------------------------------
// SensorReading / SensorEvent
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct SensorReading {
    pub sensor_id: String,
    pub value: f64,
    pub unit: String,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone)]
pub struct SensorEvent {
    pub readings: Vec<SensorReading>,
}

// ---------------------------------------------------------------------------
// SimulatedSensor
// ---------------------------------------------------------------------------

pub struct SimulatedSensor {
    pub id: String,
    base_value: f64,
    noise_amplitude: f64,
    unit: String,
    tick: u64,
}

impl SimulatedSensor {
    pub fn new(id: impl Into<String>, base_value: f64, noise_amplitude: f64, unit: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            base_value,
            noise_amplitude,
            unit: unit.into(),
            tick: 0,
        }
    }

    /// Deterministic next reading (no RNG, uses sine wave for reproducibility).
    pub fn read(&mut self) -> SensorReading {
        let noise = (self.tick as f64).sin() * self.noise_amplitude;
        self.tick += 1;
        SensorReading {
            sensor_id: self.id.clone(),
            value: self.base_value + noise,
            unit: self.unit.clone(),
            timestamp_ms: self.tick * 100,
        }
    }
}

// ---------------------------------------------------------------------------
// SensorModule
// ---------------------------------------------------------------------------

pub const SENSOR_EVENT_TYPE: &str = "sensor_event";

pub struct SensorModule {
    sensors: Vec<SimulatedSensor>,
    started: bool,
    last_readings: Vec<SensorReading>,
}

impl SensorModule {
    pub fn new() -> Self {
        Self {
            sensors: vec![
                SimulatedSensor::new("temp_01", 22.0, 1.0, "°C"),
                SimulatedSensor::new("humidity_01", 55.0, 3.0, "%"),
                SimulatedSensor::new("pressure_01", 1013.0, 5.0, "hPa"),
            ],
            started: false,
            last_readings: Vec::new(),
        }
    }

    /// Read all sensors and return a SensorEvent.
    pub fn read_all(&mut self) -> SensorEvent {
        let readings: Vec<SensorReading> = self.sensors.iter_mut().map(|s| s.read()).collect();
        self.last_readings = readings.clone();
        SensorEvent { readings }
    }

    pub fn last_readings(&self) -> &[SensorReading] {
        &self.last_readings
    }

    pub fn sensor_count(&self) -> usize {
        self.sensors.len()
    }
}

impl Default for SensorModule {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for SensorModule {
    fn name(&self) -> &str {
        "sensor"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn start(&mut self, _ctx: &mut Context) {
        self.started = true;
    }

    fn stop(&mut self) {
        self.started = false;
        self.last_readings.clear();
    }

    fn health(&self) -> HealthStatus {
        if self.started {
            HealthStatus::Healthy
        } else {
            HealthStatus::Unhealthy("SensorModule not running".into())
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
    fn simulated_sensor_read() {
        let mut s = SimulatedSensor::new("test", 10.0, 0.0, "m");
        let r = s.read();
        assert_eq!(r.sensor_id, "test");
        assert_eq!(r.value, 10.0);
        assert_eq!(r.unit, "m");
    }

    #[test]
    fn simulated_sensor_ticks() {
        let mut s = SimulatedSensor::new("t", 0.0, 1.0, "x");
        let r1 = s.read();
        let r2 = s.read();
        assert_ne!(r1.timestamp_ms, r2.timestamp_ms);
    }

    #[test]
    fn simulated_sensor_deterministic() {
        let mut s1 = SimulatedSensor::new("a", 5.0, 2.0, "y");
        let mut s2 = SimulatedSensor::new("a", 5.0, 2.0, "y");
        assert_eq!(s1.read(), s2.read());
    }

    #[test]
    fn sensor_module_new() {
        let m = SensorModule::new();
        assert_eq!(m.sensor_count(), 3);
        assert!(!m.started);
    }

    #[test]
    fn sensor_module_read_all() {
        let mut m = SensorModule::new();
        let event = m.read_all();
        assert_eq!(event.readings.len(), 3);
    }

    #[test]
    fn sensor_module_read_all_populates_last() {
        let mut m = SensorModule::new();
        m.read_all();
        assert_eq!(m.last_readings().len(), 3);
    }

    #[test]
    fn sensor_module_start_stop() {
        let mut m = SensorModule::new();
        let mut ctx = Context::new();
        m.start(&mut ctx);
        assert!(m.started);
        m.stop();
        assert!(!m.started);
        assert!(m.last_readings().is_empty());
    }

    #[test]
    fn sensor_module_health() {
        let mut m = SensorModule::new();
        let mut ctx = Context::new();
        assert!(matches!(m.health(), HealthStatus::Unhealthy(_)));
        m.start(&mut ctx);
        assert_eq!(m.health(), HealthStatus::Healthy);
    }

    #[test]
    fn sensor_module_name_version() {
        let m = SensorModule::new();
        assert_eq!(m.name(), "sensor");
        assert_eq!(m.version(), "0.1.0");
    }

    #[test]
    fn sensor_module_default() {
        let m = SensorModule::default();
        assert_eq!(m.sensor_count(), 3);
    }

    #[test]
    fn sensor_module_in_registry() {
        use openconstruct_core::ModuleRegistry;
        let mut reg = ModuleRegistry::new();
        reg.register(Box::new(SensorModule::new()));
        let mut ctx = Context::new();
        reg.start_module("sensor", &mut ctx);
        assert_eq!(reg.get("sensor").unwrap().state, ModuleState::Running);
    }

    #[test]
    fn sensor_reading_equality() {
        let a = SensorReading {
            sensor_id: "x".into(),
            value: 1.0,
            unit: "m".into(),
            timestamp_ms: 100,
        };
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn simulated_sensor_noise_applied() {
        let mut s = SimulatedSensor::new("n", 0.0, 10.0, "v");
        let r = s.read();
        // tick 1: sin(0) = 0, so value = 0.0
        // Actually tick starts at 0, first read increments to 1, uses tick=0 for calc... let me check
        // tick is incremented after calc, so first call uses tick=0, sin(0)=0
        assert_eq!(r.value, 0.0);
        // second read: tick=1, sin(1)≈0.84
        let r2 = s.read();
        assert_ne!(r2.value, 0.0);
    }
}
