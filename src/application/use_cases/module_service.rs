/// Ọ̀nụ Module Service: Application Layer
///
/// This service coordinates the registration of built-in modules
/// and external extensions into the compilation registry.

use crate::application::use_cases::registry_service::RegistryService;
use crate::domain::entities::registry::BuiltInModule;

pub struct ModuleService;

impl ModuleService {
    pub fn new() -> Self {
        Self
    }

    pub fn register_module(&self, registry: &mut RegistryService, module: &dyn BuiltInModule) {
        module.register(registry.symbols_mut());
    }
}
