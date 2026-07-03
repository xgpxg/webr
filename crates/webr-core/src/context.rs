use std::any::{Any, TypeId};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use crate::component::{Component, ComponentRegistration};
use crate::error::Error;
use crate::inject::Inject;

/// 组件工厂函数：接收容器引用以解析依赖，返回类型擦除的组件实例。
/// 使用 `FnOnce` 保证每个工厂在 `build()` 阶段仅被消费一次。
pub(crate) type FactoryFn =
    Box<dyn FnOnce(&ApplicationContext) -> Result<Box<dyn Any + Send + Sync>, Error>>;

/// IoC 容器，管理所有组件的生命周期。
///
/// 组件注册阶段仅存储工厂函数与依赖元数据，不立即实例化；
/// 调用 [`build()`](Self::build) 时通过 Kahn 拓扑排序确定构建顺序，
/// 按依赖关系依次实例化，确保被依赖组件先于依赖方就绪。
pub struct ApplicationContext {
    /// 已实例化的组件单例，`build()` 后所有组件均可通过 `TypeId` 查找
    instances: HashMap<TypeId, Arc<dyn Any + Send + Sync>>,
    /// 待消费的组件工厂函数，`build()` 过程中逐个 `remove` 消费
    factories: HashMap<TypeId, FactoryFn>,
    /// 组件依赖关系图：`type_id → 该组件所依赖的 type_id 列表`
    dep_graph: HashMap<TypeId, Vec<TypeId>>,
    /// 组件名称映射，用于错误信息中展示可读的组件名
    names: HashMap<TypeId, &'static str>,
    /// 组件注册顺序，用于拓扑排序初始化所有节点
    registration_order: Vec<TypeId>,
    /// 标记容器是否已完成构建，防止重复 `build()`
    built: bool,
}

impl ApplicationContext {
    pub fn new() -> Self {
        Self {
            instances: HashMap::new(),
            factories: HashMap::new(),
            dep_graph: HashMap::new(),
            names: HashMap::new(),
            registration_order: Vec::new(),
            built: false,
        }
    }

    /// 注册一个组件描述符，由 `#[component]` 宏生成代码调用。
    /// 仅存储工厂函数与依赖元数据，不触发实例化。
    pub fn register(&mut self, registration: ComponentRegistration) {
        let type_id = registration.type_id;
        self.names.insert(type_id, registration.name);
        self.dep_graph.insert(type_id, registration.dependencies);
        self.factories.insert(type_id, registration.factory);
        self.registration_order.push(type_id);
    }

    /// 批量注册
    pub fn register_all(&mut self, registrations: impl IntoIterator<Item = ComponentRegistration>) {
        for reg in registrations {
            self.register(reg);
        }
    }

    /// 直接注册一个已构建的实例，跳过工厂构建流程。
    /// 适用于框架内部手动注入的对象（如应用配置）。
    pub fn provide<T: Component>(&mut self, instance: T) -> Result<(), Error> {
        let type_id = TypeId::of::<T>();
        if self.instances.contains_key(&type_id) {
            return Err(Error::DuplicateComponent(T::component_name()));
        }
        self.names.insert(type_id, T::component_name());
        self.instances.insert(type_id, Arc::new(instance));
        Ok(())
    }

    // ─── 装配 API ──────────────────────────────────────────

    /// 构建所有已注册的组件。
    /// 先执行拓扑排序，再按顺序消费工厂函数完成实例化。
    /// 已通过 [`provide`](Self::provide) 注册的实例会被跳过。
    pub fn build(&mut self) -> Result<(), Error> {
        if self.built {
            return Ok(());
        }
        let order = self.topological_sort()?;

        for type_id in order {
            // 已经通过 provide 手动注册的，跳过
            if self.instances.contains_key(&type_id) {
                continue;
            }

            let factory = self
                .factories
                .remove(&type_id)
                .ok_or_else(|| Error::Internal("Missing factory during build".into()))?;

            let instance = factory(self)?;
            self.instances.insert(type_id, Arc::from(instance));
        }

        self.built = true;
        Ok(())
    }

    // ─── 解析 API ──────────────────────────────────────────

    /// 解析组件并返回 `Inject<T>` 智能指针包装。
    /// 由 `#[component]` 宏生成的构造函数在 `build()` 阶段调用。
    pub fn resolve<T: Component>(&self) -> Result<Inject<T>, Error> {
        let type_id = TypeId::of::<T>();
        let arc_any = self
            .instances
            .get(&type_id)
            .ok_or(Error::ComponentNotFound(T::component_name()))?;

        let arc_t: Arc<T> = arc_any
            .clone()
            .downcast::<T>()
            .map_err(|_| Error::DowncastFailed(T::component_name()))?;

        Ok(Inject::new(arc_t))
    }

    /// 解析组件并返回 `Arc<T>`，供框架内部使用（如控制器挂载路由）。
    pub fn resolve_arc<T: Component>(&self) -> Result<Arc<T>, Error> {
        let inject = self.resolve::<T>()?;
        Ok(inject.arc())
    }

    // ─── 内部方法 ──────────────────────────────────────────

    /// Kahn 算法拓扑排序：根据 `dep_graph` 计算组件构建顺序，
    /// 入度为 0 的节点（无依赖）优先输出。若存在环则返回错误。
    fn topological_sort(&self) -> Result<Vec<TypeId>, Error> {
        let mut in_degree: HashMap<TypeId, usize> = HashMap::new();
        let mut reverse_adj: HashMap<TypeId, Vec<TypeId>> = HashMap::new();

        // 初始化所有已注册节点的入度
        for &type_id in &self.registration_order {
            in_degree.entry(type_id).or_insert(0);
        }
        // `provide` 直接注册的实例同样参与排序（入度为 0）
        for &type_id in self.instances.keys() {
            in_degree.entry(type_id).or_insert(0);
        }

        // 构建入度表与反向邻接表（dep → 依赖它的组件列表）
        for (&type_id, deps) in &self.dep_graph {
            for &dep in deps {
                if in_degree.contains_key(&dep) {
                    *in_degree.entry(type_id).or_insert(0) += 1;
                    reverse_adj.entry(dep).or_default().push(type_id);
                }
            }
        }

        // 入度为 0 的节点（无未满足依赖）入队
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

        // 排序结果少于节点数，说明存在环
        if sorted.len() != in_degree.len() {
            return Err(Error::CircularDependency(
                "Dependency cycle detected".into(),
            ));
        }

        Ok(sorted)
    }
}

impl Default for ApplicationContext {
    fn default() -> Self {
        Self::new()
    }
}
