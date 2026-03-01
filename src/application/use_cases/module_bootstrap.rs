use crate::application::use_cases::registry_service::RegistryService;
use crate::application::use_cases::module_service::ModuleService;
use crate::application::ports::environment::EnvironmentPort;
use crate::application::options::LogLevel;
use crate::domain::entities::core_module::{CoreModule, StandardMathModule};
use crate::infrastructure::extensions::io::OnuIoModule;

pub struct ModuleBootstrap;

impl ModuleBootstrap {
    pub fn register_all<E: EnvironmentPort>(
        registry: &mut RegistryService,
        env: &E,
        log_level: LogLevel,
    ) {
        let module_service = ModuleService::new(env, log_level);

        // Register Built-in Modules
        module_service.register_module(registry, &CoreModule);
        module_service.register_module(registry, &StandardMathModule);
        module_service.register_module(registry, &OnuIoModule);
    }
}
