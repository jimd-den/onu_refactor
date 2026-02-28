/// Ọ̀nụ Module Service: Application Layer
///
/// This service coordinates the registration of built-in modules
/// and external extensions into the compilation registry.

use crate::application::use_cases::registry_service::RegistryService;
use crate::domain::entities::registry::BuiltInModule;
use crate::application::options::LogLevel;
use crate::application::ports::environment::EnvironmentPort;

pub struct ModuleService<'a> {
    pub env: &'a dyn EnvironmentPort,
    pub log_level: LogLevel,
}

impl<'a> ModuleService<'a> {
    pub fn new(env: &'a dyn EnvironmentPort, log_level: LogLevel) -> Self {
        Self { env, log_level }
    }

    fn log(&self, level: LogLevel, message: &str) {
        if level <= self.log_level {
            self.env.log(level, &format!("[ModuleService] {}", message));
        }
    }

    pub fn register_module(&self, registry: &mut RegistryService, module: &dyn BuiltInModule) {
        self.log(LogLevel::Debug, &format!("Registering module: {}", module.name()));
        module.register(registry.symbols_mut());
    }
}
