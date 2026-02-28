/// Ọ̀nụ Module Service: Application Layer
///
/// This service coordinates the registration of built-in modules
/// and external extensions into the compilation registry.

use crate::application::use_cases::registry_service::RegistryService;
use crate::domain::entities::registry::BuiltInModule;
use crate::application::options::LogLevel;
use chrono::Local;

pub struct ModuleService {
    pub log_level: LogLevel,
}

impl ModuleService {
    pub fn new(log_level: LogLevel) -> Self {
        Self { log_level }
    }

    fn log(&self, level: LogLevel, message: &str) {
        if level <= self.log_level && level != LogLevel::None {
            let timestamp = Local::now().to_rfc3339();
            eprintln!("[{}] {:?}: [ModuleService] {}", timestamp, level, message);
        }
    }

    pub fn register_module(&self, registry: &mut RegistryService, module: &dyn BuiltInModule) {
        self.log(LogLevel::Debug, &format!("Registering module: {}", module.name()));
        module.register(registry.symbols_mut());
    }
}
