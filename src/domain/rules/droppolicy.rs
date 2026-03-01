use crate::domain::entities::mir::{MirInstruction, MirOperand};
use crate::application::use_cases::mir_builder::MirBuilder;

pub struct DropPolicy;

impl DropPolicy {
    pub fn collect_resource_drop(op: &MirOperand, builder: &mut MirBuilder) {
        if let MirOperand::Variable(ssa_id, is_consuming) = op {
            if *is_consuming {
                if let Some(typ) = builder.resolve_ssa_type(*ssa_id) {
                    if typ.is_resource() {
                        builder.schedule_drop(*ssa_id, typ);
                    }
                }
            }
        }
    }

    pub fn emit_surviving_drops(builder: &mut MirBuilder) {
        for (var_id, var_typ, var_name, is_dyn) in builder.get_surviving_resources() {
            if is_dyn && !builder.is_consumed(var_id) {
                builder.emit(MirInstruction::Drop {
                    ssa_var: var_id,
                    typ: var_typ,
                    name: var_name,
                    is_dynamic: is_dyn,
                });
            }
        }
    }
}
