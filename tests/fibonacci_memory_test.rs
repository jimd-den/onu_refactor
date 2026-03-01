use onu_refactor::domain::entities::hir::*;
use onu_refactor::domain::entities::mir::*;
use onu_refactor::domain::entities::types::OnuType;
use onu_refactor::application::use_cases::mir_builder::MirBuilder;
use onu_refactor::application::use_cases::mir_lowering_service::MirLoweringService;
use onu_refactor::application::use_cases::registry_service::RegistryService;
use onu_refactor::infrastructure::os::NativeOsEnvironment;
use onu_refactor::application::options::LogLevel;

use onu_refactor::domain::entities::registry::BehaviorSignature;

fn setup_service() -> (NativeOsEnvironment, RegistryService) {
    let env = NativeOsEnvironment::new(LogLevel::Error);
    let mut registry = RegistryService::new();
    // Register needed builtins for tests
    registry.symbols_mut().add_signature("as-text", BehaviorSignature {
        input_types: vec![OnuType::I64],
        return_type: OnuType::Strings,
        arg_is_observation: vec![false],
    });
    registry.symbols_mut().add_signature("joined-with", BehaviorSignature {
        input_types: vec![OnuType::Strings, OnuType::Strings],
        return_type: OnuType::Strings,
        arg_is_observation: vec![false, false],
    });
    (env, registry)
}

fn count_drops(instructions: &[MirInstruction], ssa_id: usize) -> usize {
    instructions.iter().filter(|inst| {
        if let MirInstruction::Drop { ssa_var, .. } = inst {
            *ssa_var == ssa_id
        } else {
            false
        }
    }).count()
}

fn count_allocs(instructions: &[MirInstruction]) -> usize {
    instructions.iter().filter(|inst| matches!(inst, MirInstruction::Alloc { .. })).count()
}

fn count_all_drops(instructions: &[MirInstruction]) -> usize {
    instructions.iter().filter(|inst| matches!(inst, MirInstruction::Drop { is_dynamic: true, .. })).count()
}

// 1. Just as-text
#[test]
fn test_fibo_as_text_call() {
    let (env, registry) = setup_service();
    let service = MirLoweringService::new(&env, &registry);
    let mut builder = MirBuilder::new("test".to_string(), OnuType::Strings);

    // (40 utilizes as-text) -> Call { name: "as-text", args: [40] }
    let expr = HirExpression::Call {
        name: "as-text".to_string(),
        args: vec![HirExpression::Literal(HirLiteral::I64(40))],
    };

    let res = service.context.lower_expression(&expr, &mut builder, false).unwrap();
    
    let pending = builder.take_pending_drops();
    for (var, typ, name, is_dyn) in pending {
        if is_dyn && !builder.is_consumed(var) {
            builder.emit(MirInstruction::Drop { ssa_var: var, typ, name, is_dynamic: is_dyn });
        }
    }

    let func = builder.build();
    let insts = &func.blocks[0].instructions;
    
    println!("test_fibo_as_text_call INSTRUCTIONS:
{:#?}", insts);
    
    // as-text allocates 1 string. We expect 1 alloc and 1 drop if it's intermediate
    assert_eq!(count_allocs(insts), 1, "Should have 1 allocation");
    
    if let MirOperand::Variable(res_ssa, _) = res {
        assert_eq!(count_drops(insts, res_ssa), 1, "Result of as-text must be dropped");
    } else {
        panic!("as-text should return a variable");
    }
}

// 2. Chained joined-with (simulate: msg joined-with " has reached: ")
#[test]
fn test_fibo_joined_with_literal() {
    let (env, registry) = setup_service();
    let service = MirLoweringService::new(&env, &registry);
    let mut builder = MirBuilder::new("test".to_string(), OnuType::Strings);

    // "msg" joined-with " has reached: "
    builder.define_variable("msg", 100, OnuType::Strings);
    builder.set_ssa_type(100, OnuType::Strings);
    builder.set_ssa_is_dynamic(100, true);

    let expr = HirExpression::Call {
        name: "joined-with".to_string(),
        args: vec![
            HirExpression::Variable("msg".to_string(), false), // not consuming
            HirExpression::Literal(HirLiteral::Text(" has reached: ".to_string())),
        ],
    };

    let res = service.context.lower_expression(&expr, &mut builder, false).unwrap();
    
    let pending = builder.take_pending_drops();
    for (var, typ, name, is_dyn) in pending {
        if is_dyn && !builder.is_consumed(var) {
            builder.emit(MirInstruction::Drop { ssa_var: var, typ, name, is_dynamic: is_dyn });
        }
    }

    let func = builder.build();
    let insts = &func.blocks[0].instructions;
    
    println!("test_fibo_joined_with_literal INSTRUCTIONS:
{:#?}", insts);
    
    assert_eq!(count_allocs(insts), 1, "joined-with should allocate 1 block");
    if let MirOperand::Variable(res_ssa, _) = res {
        assert_eq!(count_drops(insts, res_ssa), 1, "Result of joined-with must be dropped");
    } else {
        panic!("joined-with should return a variable");
    }
    
    assert_eq!(count_drops(insts, 100), 0, "msg was not consuming, should not be dropped by joined-with");
}

// 3. joined-with dynamic (simulate: "The population " joined-with (target utilizes as-text))
#[test]
fn test_fibo_joined_with_dynamic() {
    let (env, registry) = setup_service();
    let service = MirLoweringService::new(&env, &registry);
    let mut builder = MirBuilder::new("test".to_string(), OnuType::Strings);

    let expr = HirExpression::Call {
        name: "joined-with".to_string(),
        args: vec![
            HirExpression::Literal(HirLiteral::Text("The population ".to_string())),
            HirExpression::Call {
                name: "as-text".to_string(),
                args: vec![HirExpression::Literal(HirLiteral::I64(40))],
            },
        ],
    };

    let res = service.context.lower_expression(&expr, &mut builder, false).unwrap();
    
    let pending = builder.take_pending_drops();
    for (var, typ, name, is_dyn) in pending {
        if is_dyn && !builder.is_consumed(var) {
            builder.emit(MirInstruction::Drop { ssa_var: var, typ, name, is_dynamic: is_dyn });
        }
    }

    let func = builder.build();
    let insts = &func.blocks[0].instructions;
    
    println!("test_fibo_joined_with_dynamic INSTRUCTIONS:
{:#?}", insts);
    
    // as-text (1 alloc) + joined-with (1 alloc) = 2 allocs
    assert_eq!(count_allocs(insts), 2, "Should have 2 allocations");
    
    // as-text result MUST be dropped by joined-with. joined-with result MUST be dropped at the end.
    assert_eq!(count_all_drops(insts), 2, "Should have 2 drops for dynamic allocations");
}

// 4. Derivation assigning a dynamic resource
#[test]
fn test_fibo_derivation_dynamic() {
    let (env, registry) = setup_service();
    let service = MirLoweringService::new(&env, &registry);
    let mut builder = MirBuilder::new("test".to_string(), OnuType::Nothing);

    // derivation: msg derives-from a string (40 utilizes as-text)
    // nothing
    let expr = HirExpression::Derivation {
        name: "msg".to_string(),
        typ: OnuType::Strings,
        value: Box::new(HirExpression::Call {
            name: "as-text".to_string(),
            args: vec![HirExpression::Literal(HirLiteral::I64(40))],
        }),
        body: Box::new(HirExpression::Literal(HirLiteral::Nothing)),
    };

    let _res = service.context.lower_expression(&expr, &mut builder, false).unwrap();
    
    // Simulated scope exit (though body would handle its own cleanup ideally, 
    // variables defined in scope are survivors until scope exit. Wait, MirBuilder doesn't drop scope variables automatically at exit_scope?
    // Let's see what happens to `msg` at block end)
    
    for (var_id, var_typ, var_name, is_dyn) in builder.get_surviving_resources() {
        if is_dyn && !builder.is_consumed(var_id) {
            builder.emit(MirInstruction::Drop { ssa_var: var_id, typ: var_typ, name: var_name, is_dynamic: is_dyn });
        }
    }

    let func = builder.build();
    let insts = &func.blocks[0].instructions;
    
    println!("test_fibo_derivation_dynamic INSTRUCTIONS:
{:#?}", insts);
    
    assert_eq!(count_allocs(insts), 1, "Should have 1 allocation for as-text");
    assert_eq!(count_all_drops(insts), 1, "Should have exactly 1 drop (either the source or the assigned msg)");
}

// 5. Emit dynamic resource
#[test]
fn test_fibo_broadcast_dynamic() {
    let (env, registry) = setup_service();
    let service = MirLoweringService::new(&env, &registry);
    let mut builder = MirBuilder::new("test".to_string(), OnuType::Nothing);

    // broadcasts (40 utilizes as-text)
    let expr = HirExpression::Emit(Box::new(HirExpression::Call {
        name: "as-text".to_string(),
        args: vec![HirExpression::Literal(HirLiteral::I64(40))],
    }));

    let _res = service.context.lower_expression(&expr, &mut builder, false).unwrap();
    
    let func = builder.build();
    let insts = &func.blocks[0].instructions;
    
    println!("test_fibo_broadcast_dynamic INSTRUCTIONS:
{:#?}", insts);
    
    assert_eq!(count_allocs(insts), 1, "Should have 1 allocation");
    assert_eq!(count_all_drops(insts), 1, "Emit should drop the resource after using it");
}
