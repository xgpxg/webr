use std::any::{Any, TypeId};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use crate::component::Component;
use crate::error::FrameworkError;
use crate::inject::Inject;

/// Component factory: resolves dependencies from the container and returns a type-erased instance.
/// Each factory is consumed exactly once during `build()`.
pub type FactoryFn<E> =
    Box<dyn FnOnce(&ApplicationContext<E>) -> Result<Box<dyn Any + Send + Sync>, E> + Send + Sync>;

/// IoC container managing component lifecycle.
///
/// Registration phase stores factory functions and dependency metadata only;
/// calling [`build()`](Self::build) triggers Kahn topological sort to determine
/// build order, then instantiates components in dependency order.
///
/// Generic over error type `E` so that:
/// - webr-web uses `ApplicationContext<Error>` (user-facing HTTP errors)
/// - Non-web consumers can use any error type
pub struct ApplicationContext<E: std::error::Error + Send + Sync + 'static> {
    /// Instantiated component singletons, keyed by TypeId
    instances: HashMap<TypeId, Arc<dyn Any + Send + Sync>>,
    /// Factory functions consumed during build
    factories: HashMap<TypeId, FactoryFn<E>>,
    /// Dependency graph: type_id → list of required type_ids
    dep_graph: HashMap<TypeId, Vec<TypeId>>,
    /// Human-readable component names for error messages
    names: HashMap<TypeId, &'static str>,
    /// Registration order for topological sort initialization
    registration_order: Vec<TypeId>,
    /// Whether build() has been called
    built: bool,
    /// Phantom data for E (required since E only appears in FactoryFn<E>)
    _error: std::marker::PhantomData<E>,
}

impl<E: std::error::Error + Send + Sync + 'static> ApplicationContext<E> {
    pub fn new() -> Self {
        Self {
            instances: HashMap::new(),
            factories: HashMap::new(),
            dep_graph: HashMap::new(),
            names: HashMap::new(),
            registration_order: Vec::new(),
            built: false,
            _error: std::marker::PhantomData,
        }
    }

    /// Register a component descriptor. Called by `#[component]` macro generated code.
    /// Stores factory function and dependency metadata without triggering instantiation.
    pub fn register(&mut self, registration: crate::component::ComponentRegistration<E>) {
        let type_id = registration.type_id;
        self.names.insert(type_id, registration.name);
        self.dep_graph.insert(type_id, registration.dependencies);
        self.factories.insert(type_id, registration.factory);
        self.registration_order.push(type_id);
    }

    /// Batch registration
    pub fn register_all(
        &mut self,
        registrations: impl IntoIterator<Item = crate::component::ComponentRegistration<E>>,
    ) {
        for reg in registrations {
            self.register(reg);
        }
    }

    /// Register a pre-built instance directly, bypassing the factory.
    /// Used for framework-internal objects (e.g., application config).
    pub fn provide<T: Component>(&mut self, instance: T) -> Result<(), FrameworkError> {
        let type_id = TypeId::of::<T>();
        if self.instances.contains_key(&type_id) {
            return Err(FrameworkError::DuplicateComponent(T::component_name()));
        }
        self.names.insert(type_id, T::component_name());
        self.instances.insert(type_id, Arc::new(instance));
        Ok(())
    }

    // ─── Build API ──────────────────────────────────────────

    /// Build all registered components.
    /// Performs topological sort, then consumes factory functions to instantiate.
    /// Instances registered via [`provide`](Self::provide) are skipped.
    pub fn build(&mut self) -> Result<(), E>
    where
        E: From<FrameworkError>,
    {
        if self.built {
            return Ok(());
        }
        let order = self.topological_sort()?;

        for type_id in order {
            if self.instances.contains_key(&type_id) {
                continue;
            }

            let factory = self
                .factories
                .remove(&type_id)
                .ok_or_else(|| FrameworkError::ConfigError("Missing factory during build".into()))?;

            let instance = factory(self)?;
            self.instances.insert(type_id, Arc::from(instance));
        }

        self.built = true;
        Ok(())
    }

    // ─── Resolve API ──────────────────────────────────────────

    /// Resolve a component and return an `Inject<T>` smart pointer.
    /// Called by `#[component]` macro generated constructors during `build()`.
    pub fn resolve<T: Component>(&self) -> Result<Inject<T>, E>
    where
        E: From<FrameworkError>,
    {
        let type_id = TypeId::of::<T>();
        let arc_any = self
            .instances
            .get(&type_id)
            .ok_or_else(|| FrameworkError::ComponentNotFound(T::component_name()))?;

        let arc_t: Arc<T> = arc_any
            .clone()
            .downcast::<T>()
            .map_err(|_| FrameworkError::DowncastFailed(T::component_name()))?;

        Ok(Inject::new(arc_t))
    }

    /// Resolve and return `Arc<T>`, used internally by the framework (e.g., mounting controller routes).
    pub fn resolve_arc<T: Component>(&self) -> Result<Arc<T>, E>
    where
        E: From<FrameworkError>,
    {
        let inject = self.resolve::<T>()?;
        Ok(inject.arc())
    }

    // ─── Internal methods ──────────────────────────────────────────

    /// Kahn's algorithm topological sort: determines component build order.
    /// Nodes with in-degree 0 (no dependencies) are processed first.
    /// Returns error if a cycle is detected.
    fn topological_sort(&self) -> Result<Vec<TypeId>, FrameworkError> {
        let mut in_degree: HashMap<TypeId, usize> = HashMap::new();
        let mut reverse_adj: HashMap<TypeId, Vec<TypeId>> = HashMap::new();

        for &type_id in &self.registration_order {
            in_degree.entry(type_id).or_insert(0);
        }
        for &type_id in self.instances.keys() {
            in_degree.entry(type_id).or_insert(0);
        }

        for (&type_id, deps) in &self.dep_graph {
            for &dep in deps {
                if in_degree.contains_key(&dep) {
                    *in_degree.entry(type_id).or_insert(0) += 1;
                    reverse_adj.entry(dep).or_default().push(type_id);
                }
            }
        }

        let mut queue: VecDeque<TypeId> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&id, _)| id)
            .collect();

        let mut sorted = Vec::with_capacity(in_degree.len());

        while let Some(current) = queue.pop_front() {
            sorted.push(current);
            if let Some(dependents) = reverse_adj.get(&current) {
                for &dependent in dependents {
                    if let Some(deg) = in_degree.get_mut(&dependent) {
                        *deg = deg.saturating_sub(1);
                        if *deg == 0 {
                            queue.push_back(dependent);
                        }
                    }
                }
            }
        }

        if sorted.len() != in_degree.len() {
            let cycle_names: Vec<&str> = in_degree
                .keys()
                .filter(|id| !sorted.contains(id))
                .filter_map(|id| self.names.get(id).copied())
                .collect();
            return Err(FrameworkError::CircularDependency(format!(
                "Dependency cycle detected among: {}",
                cycle_names.join(", ")
            )));
        }

        Ok(sorted)
    }


}

impl<E: std::error::Error + Send + Sync + 'static> Default for ApplicationContext<E> {
    fn default() -> Self {
        Self::new()
    }
}
