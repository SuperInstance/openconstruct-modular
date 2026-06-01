#![deny(unsafe_code)]

use std::any::Any;
use std::collections::HashMap;
use std::fmt;

// ---------------------------------------------------------------------------
// ModuleState
// ---------------------------------------------------------------------------

/// Lifecycle state of a module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModuleState {
    Stopped,
    Starting,
    Running,
    Stopping,
    Failed,
}

impl fmt::Display for ModuleState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModuleState::Stopped => write!(f, "Stopped"),
            ModuleState::Starting => write!(f, "Starting"),
            ModuleState::Running => write!(f, "Running"),
            ModuleState::Stopping => write!(f, "Stopping"),
            ModuleState::Failed => write!(f, "Failed"),
        }
    }
}

// ---------------------------------------------------------------------------
// HealthStatus
// ---------------------------------------------------------------------------

/// Health check result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Degraded(String),
    Unhealthy(String),
}

// ---------------------------------------------------------------------------
// Context – shared state bag
// ---------------------------------------------------------------------------

/// A type-erased key-value store shared across modules.
pub struct Context {
    store: HashMap<String, Box<dyn Any + Send + Sync>>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            store: HashMap::new(),
        }
    }

    /// Insert a value.
    pub fn set<T: Any + Send + Sync + 'static>(&mut self, key: impl Into<String>, value: T) {
        self.store.insert(key.into(), Box::new(value));
    }

    /// Get a shared reference.
    pub fn get<T: Any + Send + Sync + 'static>(&self, key: &str) -> Option<&T> {
        self.store.get(key).and_then(|v| v.downcast_ref::<T>())
    }

    /// Get a mutable reference.
    pub fn get_mut<T: Any + Send + Sync + 'static>(&mut self, key: &str) -> Option<&mut T> {
        self.store.get_mut(key).and_then(|v| v.downcast_mut::<T>())
    }

    /// Remove a key.
    pub fn remove(&mut self, key: &str) {
        self.store.remove(key);
    }

    /// Check if key exists.
    pub fn contains(&self, key: &str) -> bool {
        self.store.contains_key(key)
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.store.len()
    }

    pub fn is_empty(&self) -> bool {
        self.store.is_empty()
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Module trait
// ---------------------------------------------------------------------------

/// The core trait every plug-in module must implement.
pub trait Module: Send {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn start(&mut self, ctx: &mut Context);
    fn stop(&mut self);
    fn health(&self) -> HealthStatus;
}

// ---------------------------------------------------------------------------
// EventBus
// ---------------------------------------------------------------------------

/// Type-alias for an event handler closure.
pub type HandlerFn = Box<dyn FnMut(&dyn Any) + Send>;

/// Simple string-keyed event bus.
pub struct EventBus {
    handlers: HashMap<String, Vec<HandlerFn>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    /// Subscribe a handler to an event type.
    pub fn subscribe(&mut self, event_type: impl Into<String>, handler: HandlerFn) {
        self.handlers
            .entry(event_type.into())
            .or_default()
            .push(handler);
    }

    /// Publish an event, calling every handler subscribed to its type.
    pub fn publish(&mut self, event_type: &str, event: &dyn Any) {
        if let Some(handlers) = self.handlers.get_mut(event_type) {
            for h in handlers.iter_mut() {
                h(event);
            }
        }
    }

    /// Remove all handlers for an event type.
    pub fn unsubscribe(&mut self, event_type: &str) {
        self.handlers.remove(event_type);
    }

    /// Number of handler groups.
    pub fn event_type_count(&self) -> usize {
        self.handlers.len()
    }

    /// Number of handlers for a specific event type.
    pub fn handler_count(&self, event_type: &str) -> usize {
        self.handlers.get(event_type).map_or(0, |v| v.len())
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// ModuleRegistry
// ---------------------------------------------------------------------------

/// Entry holding a module plus its current lifecycle state.
pub struct ModuleEntry {
    pub module: Box<dyn Module>,
    pub state: ModuleState,
}

/// Central registry for loaded modules.
pub struct ModuleRegistry {
    modules: HashMap<String, ModuleEntry>,
}

impl ModuleRegistry {
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
        }
    }

    /// Register (add) a new module.
    pub fn register(&mut self, module: Box<dyn Module>) {
        let id = module.name().to_owned();
        self.modules.insert(
            id,
            ModuleEntry {
                module,
                state: ModuleState::Stopped,
            },
        );
    }

    /// Remove a module by id.
    pub fn unregister(&mut self, module_id: &str) -> Option<Box<dyn Module>> {
        self.modules.remove(module_id).map(|e| e.module)
    }

    /// Get a reference to a registered module entry.
    pub fn get(&self, module_id: &str) -> Option<&ModuleEntry> {
        self.modules.get(module_id)
    }

    /// Get a mutable reference.
    pub fn get_mut(&mut self, module_id: &str) -> Option<&mut ModuleEntry> {
        self.modules.get_mut(module_id)
    }

    /// Number of registered modules.
    pub fn len(&self) -> usize {
        self.modules.len()
    }

    pub fn is_empty(&self) -> bool {
        self.modules.is_empty()
    }

    /// Iterate module ids.
    pub fn module_ids(&self) -> Vec<&str> {
        self.modules.keys().map(|s| s.as_str()).collect()
    }

    /// Start all modules in insertion order.
    pub fn start_all(&mut self, ctx: &mut Context) {
        let ids: Vec<String> = self.modules.keys().cloned().collect();
        for id in ids {
            self.start_module(&id, ctx);
        }
    }

    /// Stop all modules.
    pub fn stop_all(&mut self) {
        let ids: Vec<String> = self.modules.keys().cloned().collect();
        for id in ids {
            self.stop_module(&id);
        }
    }

    /// Start a single module.
    pub fn start_module(&mut self, module_id: &str, ctx: &mut Context) {
        if let Some(entry) = self.modules.get_mut(module_id) {
            entry.state = ModuleState::Starting;
            entry.module.start(ctx);
            entry.state = ModuleState::Running;
        }
    }

    /// Stop a single module.
    pub fn stop_module(&mut self, module_id: &str) {
        if let Some(entry) = self.modules.get_mut(module_id) {
            entry.state = ModuleState::Stopping;
            entry.module.stop();
            entry.state = ModuleState::Stopped;
        }
    }
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests (core crate)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- ModuleState --
    #[test]
    fn module_state_display() {
        assert_eq!(format!("{}", ModuleState::Stopped), "Stopped");
        assert_eq!(format!("{}", ModuleState::Starting), "Starting");
        assert_eq!(format!("{}", ModuleState::Running), "Running");
        assert_eq!(format!("{}", ModuleState::Stopping), "Stopping");
        assert_eq!(format!("{}", ModuleState::Failed), "Failed");
    }

    #[test]
    fn module_state_equality() {
        assert_eq!(ModuleState::Running, ModuleState::Running);
        assert_ne!(ModuleState::Stopped, ModuleState::Running);
    }

    // -- HealthStatus --
    #[test]
    fn health_status_equality() {
        assert_eq!(HealthStatus::Healthy, HealthStatus::Healthy);
        assert_ne!(HealthStatus::Healthy, HealthStatus::Unhealthy("err".into()));
    }

    // -- Context --
    #[test]
    fn context_set_get() {
        let mut ctx = Context::new();
        ctx.set("count", 42u32);
        assert_eq!(*ctx.get::<u32>("count").unwrap(), 42);
    }

    #[test]
    fn context_missing_key() {
        let ctx = Context::new();
        assert!(ctx.get::<u32>("nope").is_none());
    }

    #[test]
    fn context_remove() {
        let mut ctx = Context::new();
        ctx.set("x", 1i32);
        ctx.remove("x");
        assert!(!ctx.contains("x"));
    }

    #[test]
    fn context_len_and_empty() {
        let mut ctx = Context::new();
        assert!(ctx.is_empty());
        ctx.set("a", 1u8);
        assert_eq!(ctx.len(), 1);
        assert!(!ctx.is_empty());
    }

    #[test]
    fn context_overwrite() {
        let mut ctx = Context::new();
        ctx.set("v", 10u32);
        ctx.set("v", 20u32);
        assert_eq!(*ctx.get::<u32>("v").unwrap(), 20);
    }

    #[test]
    fn context_different_types() {
        let mut ctx = Context::new();
        ctx.set("n", 1u32);
        ctx.set("s", String::from("hello"));
        assert_eq!(*ctx.get::<u32>("n").unwrap(), 1);
        assert_eq!(ctx.get::<String>("s").unwrap(), "hello");
    }

    #[test]
    fn context_get_mut() {
        let mut ctx = Context::new();
        ctx.set("v", 5u32);
        *ctx.get_mut::<u32>("v").unwrap() = 99;
        assert_eq!(*ctx.get::<u32>("v").unwrap(), 99);
    }

    #[test]
    fn context_default() {
        let ctx = Context::default();
        assert!(ctx.is_empty());
    }

    // -- EventBus --
    #[test]
    fn eventbus_subscribe_and_publish() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let counter = std::sync::Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();
        let mut bus = EventBus::new();
        bus.subscribe("ping", Box::new(move |_: &dyn Any| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        }));
        bus.publish("ping", &42u32);
        bus.publish("ping", &"hello");
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn eventbus_unsubscribe() {
        let mut bus = EventBus::new();
        bus.subscribe("e", Box::new(|_: &dyn Any| {}));
        bus.unsubscribe("e");
        assert_eq!(bus.handler_count("e"), 0);
    }

    #[test]
    fn eventbus_handler_count() {
        let mut bus = EventBus::new();
        assert_eq!(bus.handler_count("x"), 0);
        bus.subscribe("x", Box::new(|_: &dyn Any| {}));
        assert_eq!(bus.handler_count("x"), 1);
        bus.subscribe("x", Box::new(|_: &dyn Any| {}));
        assert_eq!(bus.handler_count("x"), 2);
    }

    #[test]
    fn eventbus_publish_no_handlers() {
        let mut bus = EventBus::new();
        bus.publish("nothing", &()); // should not panic
    }

    #[test]
    fn eventbus_event_type_count() {
        let mut bus = EventBus::new();
        assert_eq!(bus.event_type_count(), 0);
        bus.subscribe("a", Box::new(|_: &dyn Any| {}));
        bus.subscribe("b", Box::new(|_: &dyn Any| {}));
        assert_eq!(bus.event_type_count(), 2);
    }

    #[test]
    fn eventbus_default() {
        let bus = EventBus::default();
        assert_eq!(bus.event_type_count(), 0);
    }

    // -- ModuleRegistry --
    struct DummyModule;
    impl Module for DummyModule {
        fn name(&self) -> &str { "dummy" }
        fn version(&self) -> &str { "0.1.0" }
        fn start(&mut self, _ctx: &mut Context) {}
        fn stop(&mut self) {}
        fn health(&self) -> HealthStatus { HealthStatus::Healthy }
    }

    #[test]
    fn registry_register_and_get() {
        let mut reg = ModuleRegistry::new();
        reg.register(Box::new(DummyModule));
        let entry = reg.get("dummy").unwrap();
        assert_eq!(entry.module.name(), "dummy");
        assert_eq!(entry.state, ModuleState::Stopped);
    }

    #[test]
    fn registry_unregister() {
        let mut reg = ModuleRegistry::new();
        reg.register(Box::new(DummyModule));
        let removed = reg.unregister("dummy");
        assert!(removed.is_some());
        assert!(reg.get("dummy").is_none());
    }

    #[test]
    fn registry_start_stop_module() {
        let mut reg = ModuleRegistry::new();
        reg.register(Box::new(DummyModule));
        let mut ctx = Context::new();
        reg.start_module("dummy", &mut ctx);
        assert_eq!(reg.get("dummy").unwrap().state, ModuleState::Running);
        reg.stop_module("dummy");
        assert_eq!(reg.get("dummy").unwrap().state, ModuleState::Stopped);
    }

    #[test]
    fn registry_len_and_empty() {
        let mut reg = ModuleRegistry::new();
        assert!(reg.is_empty());
        reg.register(Box::new(DummyModule));
        assert_eq!(reg.len(), 1);
    }

    #[test]
    fn registry_module_ids() {
        let mut reg = ModuleRegistry::new();
        reg.register(Box::new(DummyModule));
        let ids = reg.module_ids();
        assert!(ids.contains(&"dummy"));
    }

    #[test]
    fn registry_get_missing() {
        let reg = ModuleRegistry::new();
        assert!(reg.get("nope").is_none());
    }

    #[test]
    fn registry_default() {
        let reg = ModuleRegistry::default();
        assert!(reg.is_empty());
    }
}
