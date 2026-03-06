pub mod compound_memo_strategy;
pub mod primitive_memo_strategy;

use crate::domain::entities::mir::MirFunction;

use crate::application::use_cases::registry_service::RegistryService;

pub trait MemoStrategy {
    fn create_wrapper_and_inner(
        &self,
        func: MirFunction,
        cache_size: usize,
        registry: &RegistryService,
    ) -> (MirFunction, MirFunction);
}

pub fn max_ssa_in_function(func: &MirFunction) -> usize {
    let mut max = func.args.iter().map(|a| a.ssa_var).max().unwrap_or(0);
    for block in &func.blocks {
        for inst in &block.instructions {
            let dest: Option<usize> = match inst {
                crate::domain::entities::mir::MirInstruction::Assign { dest, .. } => Some(*dest),
                crate::domain::entities::mir::MirInstruction::BinaryOperation { dest, .. } => {
                    Some(*dest)
                }
                crate::domain::entities::mir::MirInstruction::Call { dest, .. } => Some(*dest),
                crate::domain::entities::mir::MirInstruction::Tuple { dest, .. } => Some(*dest),
                crate::domain::entities::mir::MirInstruction::Index { dest, .. } => Some(*dest),
                crate::domain::entities::mir::MirInstruction::Alloc { dest, .. } => Some(*dest),
                crate::domain::entities::mir::MirInstruction::PointerOffset { dest, .. } => {
                    Some(*dest)
                }
                crate::domain::entities::mir::MirInstruction::Load { dest, .. } => Some(*dest),
                _ => None,
            };
            if let Some(d) = dest {
                if d > max {
                    max = d;
                }
            }
        }
    }
    max
}
