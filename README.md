# openconstruct-modular

> Modular plugin architecture for composable AI agent systems

## What This Does

openconstruct-modular is a Rust workspace that provides a modular, plugin-based framework for building AI agent systems. It defines a core plugin trait system, ships ready-made modules for sensor input, mathematical reasoning, and agent orchestration, and includes a runtime that wires everything together with hot-swappable capabilities.

## The Key Idea

Think of it like a Linux kernel module system, but for AI agents. Each module (plugin) implements a common `Plugin` trait with lifecycle hooks — `activate`, `deactivate`, `execute`. The runtime discovers them, resolves dependencies, and runs them in order. You can swap out a math module for a different one without touching the rest of the pipeline.

## Install

```toml
[dependencies]
openconstruct-modular = { git = "https://github.com/SuperInstance/openconstruct-modular" }
```

## Quick Start

```rust
use openconstruct_modular::core::{Plugin, PluginContext, PluginMetadata, PluginState};
use openconstruct_modular::runtime::Runtime;

// Define a custom plugin
struct MyPlugin;

impl Plugin for MyPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "my-plugin".into(),
            version: "0.1.0".into(),
            description: "Does something useful".into(),
        }
    }

    fn activate(&mut self, _ctx: &mut PluginContext) -> Result<(), String> {
        Ok(())
    }

    fn execute(&mut self, _ctx: &mut PluginContext) -> Result<(), String> {
        println!("Plugin executing!");
        Ok(())
    }

    fn deactivate(&mut self, _ctx: &mut PluginContext) -> Result<(), String> {
        Ok(())
    }
}

// Run it
fn main() {
    let mut runtime = Runtime::new();
    runtime.register(Box::new(MyPlugin));
    runtime.start().unwrap();
    runtime.execute_all().unwrap();
    runtime.shutdown().unwrap();
}
```

## API Reference

### Core Crate (`openconstruct-core`)

| Type | Description |
|------|-------------|
| `Plugin` | Core trait. Implement `metadata()`, `activate()`, `execute()`, `deactivate()`. |
| `PluginContext` | Execution context passed to plugins — provides shared state and configuration. |
| `PluginMetadata` | Struct with `name`, `version`, `description` fields. |
| `PluginState` | Enum: `Inactive`, `Active`, `Error(String)`. |
| `PluginRegistry` | Stores and manages registered plugins. Supports `register()`, `get()`, `get_mut()`, `list()`. |

### Tech-Math (`openconstruct-tech-math`)

Mathematical reasoning module. Provides spectral analysis, tensor operations, and norm computations as a plugin.

### Tech-Sensor (`openconstruct-tech-sensor`)

Sensor integration module. Wraps sensor data ingestion into the plugin pipeline.

### Tech-Agent (`openconstruct-tech-agent`)

Agent orchestration module. Manages agent lifecycle within the plugin framework.

### Runtime (`openconstruct-runtime`)

| Method | Description |
|--------|-------------|
| `Runtime::new()` | Create a new runtime. |
| `register(plugin)` | Register a boxed plugin. |
| `start()` | Activate all registered plugins. |
| `execute_all()` | Execute all active plugins in order. |
| `shutdown()` | Deactivate all plugins and clean up. |

## How It Works

1. **Registration**: Plugins are registered with the runtime's `PluginRegistry`.
2. **Activation**: `start()` calls `activate()` on each plugin, transitioning it to `Active` state.
3. **Execution**: `execute_all()` iterates plugins in registration order, calling `execute()`.
4. **Shutdown**: `shutdown()` calls `deactivate()` on each plugin.

The `PluginContext` acts as a shared mutable state bag that plugins read from and write to during execution, enabling inter-plugin communication.

## Testing

74 tests covering:
- Plugin lifecycle (activate → execute → deactivate)
- Registry operations (register, lookup, list)
- Runtime orchestration (start, execute, shutdown)
- Error handling and state transitions

## License

MIT
