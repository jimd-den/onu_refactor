use crate::application::use_cases::memo_strategies::{
    MemoStrategy, compound_memo_strategy::CompoundMemoStrategy,
    primitive_memo_strategy::PrimitiveMemoStrategy,
};
use crate::domain::entities::mir::{MirFunction, MirProgram};
use crate::domain::entities::types::OnuType;

use crate::application::use_cases::registry_service::RegistryService;

pub struct MemoPass;

const DEFAULT_MEMO_CACHE_SIZE: usize = 10000;

impl MemoPass {
    pub fn run(program: MirProgram, registry: &RegistryService) -> MirProgram {
        let mut new_functions = vec![];
        for func in program.functions {
            if Self::is_memoizable(&func) {
                let strategy: Box<dyn MemoStrategy> = match func.return_type {
                    OnuType::I64 | OnuType::Boolean | OnuType::Nothing | OnuType::Ptr => {
                        Box::new(PrimitiveMemoStrategy)
                    }
                    _ => Box::new(CompoundMemoStrategy),
                };
                let (wrapper, inner) =
                    strategy.create_wrapper_and_inner(func, DEFAULT_MEMO_CACHE_SIZE, registry);
                new_functions.push(wrapper);
                new_functions.push(inner);
            } else {
                new_functions.push(func);
            }
        }
        MirProgram {
            functions: new_functions,
        }
    }

    fn is_memoizable(func: &MirFunction) -> bool {
        let r = func.is_pure_data_leaf
            && func.diminishing.is_some()
            && func.args.len() == 1
            && func.args[0].typ == OnuType::I64;
        eprintln!(
            "[MemoPass] fn='{}' pure={} dim={:?} args={} arg0={:?} => {}",
            func.name,
            func.is_pure_data_leaf,
            func.diminishing,
            func.args.len(),
            func.args.first().map(|a| &a.typ),
            r
        );
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::mir::{BasicBlock, MirArgument, MirOperand, MirTerminator};

    #[test]
    fn test_compound_memo_strategy_panics_for_tuple_return() {
        let func = MirFunction {
            name: "test_compound".to_string(),
            args: vec![MirArgument {
                name: "x".to_string(),
                typ: OnuType::I64,
                ssa_var: 0,
            }],
            return_type: OnuType::Tuple(vec![OnuType::I64, OnuType::I64]),
            blocks: vec![BasicBlock {
                id: 0,
                instructions: vec![],
                terminator: MirTerminator::Return(MirOperand::Variable(0, false)),
            }],
            is_pure_data_leaf: true,
            diminishing: Some("x".to_string()),
        };

        let program = MirProgram {
            functions: vec![func],
        };

        let registry = RegistryService::new();
        let program = MemoPass::run(program, &registry);
        assert_eq!(program.functions.len(), 2);
    }
}
