use crate::application::use_cases::memo_strategies::{
    MemoStrategy, compound_memo_strategy::CompoundMemoStrategy,
    primitive_memo_strategy::PrimitiveMemoStrategy,
};
use crate::domain::entities::mir::{MirFunction, MirProgram};
use crate::domain::entities::types::OnuType;
use std::collections::HashSet;

use crate::application::use_cases::registry_service::RegistryService;

pub struct MemoPass;

const DEFAULT_MEMO_CACHE_SIZE: usize = 10000;

impl MemoPass {
    pub fn run(program: MirProgram, registry: &RegistryService) -> MirProgram {
        let mut new_functions = vec![];
        for func in program.functions {
            if Self::is_memoizable(&func) {
                // Multi-dimensional functions use CompoundMemoStrategy for flattened indexing.
                // 1-D functions use PrimitiveMemoStrategy for primitive return types.
                let strategy: Box<dyn MemoStrategy> = if func.diminishing.len() > 1 {
                    Box::new(CompoundMemoStrategy)
                } else {
                    match func.return_type {
                        OnuType::I64 | OnuType::Boolean | OnuType::Ptr | OnuType::WideInt(_) => {
                            Box::new(PrimitiveMemoStrategy)
                        }
                        _ => Box::new(CompoundMemoStrategy),
                    }
                };
                // Use function-specific cache size when set (e.g. by IntegerUpgradePass
                // to cap arena usage for large WideInt entries), else fall back to the
                // global default.
                let cache_size = func.memo_cache_size.unwrap_or(DEFAULT_MEMO_CACHE_SIZE);
                let (wrapper, inner) =
                    strategy.create_wrapper_and_inner(func, cache_size, registry);
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
        // Nothing (void) functions have no return value to cache and size_of(Nothing) == 0,
        // which would produce a zero-byte arena and invalid memory accesses in the cache.
        let dim = &func.diminishing;
        // Build a set for O(1) per-arg lookup, avoiding O(n²) in the all() call below.
        let dim_set: HashSet<&str> = dim.iter().map(String::as_str).collect();
        let r = func.return_type != OnuType::Nothing
            && func.is_pure_data_leaf
            && !dim.is_empty()
            && !func.args.is_empty()
            && func.args.len() == dim.len()
            && func.args.iter().all(|a| a.typ == OnuType::I64)
            && func.args.iter().all(|a| dim_set.contains(a.name.as_str()));
        eprintln!(
            "[MemoPass] fn='{}' pure={} dim={:?} args={} ret={:?} => {}",
            func.name,
            func.is_pure_data_leaf,
            func.diminishing,
            func.args.len(),
            func.return_type,
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
            diminishing: vec!["x".to_string()],
            memo_cache_size: None,
        };

        let program = MirProgram {
            functions: vec![func],
        };

        let registry = RegistryService::new();
        let program = MemoPass::run(program, &registry);
        assert_eq!(program.functions.len(), 2);
    }

    #[test]
    fn test_nothing_return_type_is_not_memoized() {
        // Functions returning Nothing (void) must not be memoized:
        // size_of(Nothing) == 0 would allocate a zero-byte cache and trigger
        // invalid memory accesses in the load/store codegen paths.
        let func = MirFunction {
            name: "void_fn".to_string(),
            args: vec![MirArgument {
                name: "x".to_string(),
                typ: OnuType::I64,
                ssa_var: 0,
            }],
            return_type: OnuType::Nothing,
            blocks: vec![BasicBlock {
                id: 0,
                instructions: vec![],
                terminator: MirTerminator::Return(MirOperand::Variable(0, false)),
            }],
            is_pure_data_leaf: true,
            diminishing: vec!["x".to_string()],
            memo_cache_size: None,
        };

        let program = MirProgram {
            functions: vec![func],
        };

        let registry = RegistryService::new();
        let program = MemoPass::run(program, &registry);
        // The function must pass through unchanged (no wrapper/inner split).
        assert_eq!(program.functions.len(), 1);
        assert_eq!(program.functions[0].name, "void_fn");
    }
}
