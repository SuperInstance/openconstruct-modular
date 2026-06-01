#![deny(unsafe_code)]

use openconstruct_core::{Context, Module, ModuleRegistry};
use openconstruct_tech_agent::SimpleAgent;
use openconstruct_tech_math::MathModule;
use openconstruct_tech_sensor::SensorModule;

fn main() {
    let mut registry = ModuleRegistry::new();
    let mut ctx = Context::new();

    // Register modules
    registry.register(Box::new(MathModule::new()));
    registry.register(Box::new(SensorModule::new()));
    registry.register(Box::new(SimpleAgent::new("runtime-agent")));

    println!("openConstruct Runtime");
    println!("Registered {} modules: {:?}", registry.len(), registry.module_ids());

    // Start all
    registry.start_all(&mut ctx);
    println!("All modules started.");

    // Print health
    for id in registry.module_ids() {
        let entry = registry.get(id).unwrap();
        println!("  {} v{} — {:?} — {:?}", 
            entry.module.name(),
            entry.module.version(),
            entry.state,
            entry.module.health(),
        );
    }

    // Run a sensor read cycle
    {
        let sensor = registry.get_mut("sensor").unwrap();
        let _sensor_mod = sensor.module.as_mut() as &mut dyn openconstruct_core::Module;
        // We need to downcast — but since SensorModule is behind Box<dyn Module>,
        // let's just demonstrate the event flow using the context approach.
    }

    // Use the sensor module directly for the demo
    // In a real system, modules would communicate via EventBus
    let mut sensor = openconstruct_tech_sensor::SensorModule::new();
    let mut agent = SimpleAgent::new("demo-agent");
    let mut demo_ctx = Context::new();
    sensor.start(&mut demo_ctx);
    agent.start(&mut demo_ctx);

    let event = sensor.read_all();
    println!("\nSensor readings:");
    for r in &event.readings {
        println!("  {} = {} {}", r.sensor_id, r.value, r.unit);
    }

    let mut engine = openconstruct_tech_math::MathEngine::new();
    let actions = agent.process_readings(&event, &mut engine);
    println!("\nAgent actions:");
    for a in &actions {
        println!("  [{}] {} (data: {:.2})", a.action_type, a.description, a.data);
    }

    // Graceful shutdown
    registry.stop_all();
    println!("\nAll modules stopped. Goodbye!");
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use openconstruct_core::{Context, Module, ModuleRegistry, ModuleState};
    use openconstruct_tech_agent::SimpleAgent;
    use openconstruct_tech_math::{MathEngine, MathModule};
    use openconstruct_tech_sensor::SensorModule;

    #[test]
    fn runtime_full_lifecycle() {
        let mut reg = ModuleRegistry::new();
        let mut ctx = Context::new();

        reg.register(Box::new(MathModule::new()));
        reg.register(Box::new(SensorModule::new()));
        reg.register(Box::new(SimpleAgent::new("test")));

        assert_eq!(reg.len(), 3);
        reg.start_all(&mut ctx);

        for id in reg.module_ids() {
            assert_eq!(reg.get(&id).unwrap().state, ModuleState::Running);
        }

        reg.stop_all();
        for id in reg.module_ids() {
            assert_eq!(reg.get(&id).unwrap().state, ModuleState::Stopped);
        }
    }

    #[test]
    fn runtime_math_in_context() {
        let mut reg = ModuleRegistry::new();
        let mut ctx = Context::new();
        reg.register(Box::new(MathModule::new()));
        reg.start_all(&mut ctx);

        let engine = ctx.get_mut::<MathEngine>("math_engine").unwrap();
        assert_eq!(engine.add(1.0, 2.0), 3.0);
    }

    #[test]
    fn runtime_cross_module_communication() {
        let mut reg = ModuleRegistry::new();
        let mut ctx = Context::new();
        reg.register(Box::new(MathModule::new()));
        reg.register(Box::new(SensorModule::new()));
        reg.register(Box::new(SimpleAgent::new("x")));

        reg.start_all(&mut ctx);

        // Simulate the workflow
        let mut sensor = SensorModule::new();
        let mut agent = SimpleAgent::new("x");
        sensor.start(&mut Context::new());
        agent.start(&mut Context::new());

        let event = sensor.read_all();
        let engine = ctx.get_mut::<MathEngine>("math_engine").unwrap();
        let actions = agent.process_readings(&event, engine);

        assert_eq!(actions.len(), 1);
    }

    #[test]
    fn runtime_unregister_during_run() {
        let mut reg = ModuleRegistry::new();
        let mut ctx = Context::new();
        reg.register(Box::new(MathModule::new()));
        reg.register(Box::new(SensorModule::new()));

        reg.start_all(&mut ctx);
        reg.stop_module("sensor");
        let removed = reg.unregister("sensor");
        assert!(removed.is_some());
        assert_eq!(reg.len(), 1);
    }

    #[test]
    fn runtime_empty_registry() {
        let mut reg = ModuleRegistry::new();
        let mut ctx = Context::new();
        reg.start_all(&mut ctx); // should be a no-op
        reg.stop_all(); // should be a no-op
        assert!(reg.is_empty());
    }
}
