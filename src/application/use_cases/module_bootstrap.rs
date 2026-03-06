use crate::application::use_cases::registry_service::RegistryService;
use crate::application::use_cases::module_service::ModuleService;
use crate::application::ports::environment::EnvironmentPort;
use crate::application::options::LogLevel;
use crate::domain::entities::core_module::{CoreModule, StandardMathModule};
use crate::domain::entities::registry::BuiltInModule;

pub struct ModuleBootstrap;

impl ModuleBootstrap {
    /// Register core (domain-level) modules plus any additional infrastructure
    /// modules supplied by the composition root.  The caller is responsible for
    /// passing infrastructure-layer modules (e.g. `OnuIoModule`) so that the
    /// application layer remains decoupled from infrastructure.
    pub fn register_all<E: EnvironmentPort>(
        registry: &mut RegistryService,
        env: &E,
        log_level: LogLevel,
        extra_modules: &[&dyn BuiltInModule],
    ) {
        let module_service = ModuleService::new(env, log_level);

        // Register core domain modules
        module_service.register_module(registry, &CoreModule);
        module_service.register_module(registry, &StandardMathModule);

        // Register infrastructure modules provided by the composition root
        for module in extra_modules {
            module_service.register_module(registry, *module);
        }
    }
}
