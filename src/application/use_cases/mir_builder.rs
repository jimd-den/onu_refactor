/// MIR Builder: Application Layer Helper
///
/// This struct encapsulates the state and logic for constructing a single MIR function.
/// It handles block management, SSA variable generation, and variable scoping.

use crate::domain::entities::mir::{MirFunction, BasicBlock, MirInstruction, MirTerminator};
use crate::domain::entities::types::OnuType;
use std::collections::HashMap;

pub struct MirBuilder {
    name: String,
    return_type: OnuType,
    args: Vec<crate::domain::entities::mir::MirArgument>,
    blocks: Vec<BasicBlock>,
    current_block_idx: Option<usize>,
    next_ssa: usize,
    scopes: Vec<HashMap<String, (usize, OnuType)>>,
    pending_drops: Vec<(usize, OnuType, String, bool)>,
    consumed_vars: std::collections::HashSet<usize>,
    ssa_types: HashMap<usize, OnuType>,
    ssa_is_dynamic: HashMap<usize, bool>,
    survivors: HashMap<usize, (String, bool)>,
}

impl MirBuilder {
    pub fn new(name: String, return_type: OnuType) -> Self {
        let entry_block = BasicBlock {
            id: 0,
            instructions: Vec::new(),
            terminator: MirTerminator::Unreachable,
        };

        Self {
            name,
            return_type,
            args: Vec::new(),
            blocks: vec![entry_block],
            current_block_idx: Some(0),
            next_ssa: 0,
            scopes: vec![HashMap::new()],
            pending_drops: Vec::new(),
            consumed_vars: std::collections::HashSet::new(),
            ssa_types: HashMap::new(),
            ssa_is_dynamic: HashMap::new(),
            survivors: HashMap::new(),
        }
    }

    pub fn add_arg(&mut self, name: String, typ: OnuType, ssa_var: usize) {
        self.ssa_types.insert(ssa_var, typ.clone());
        self.ssa_is_dynamic.insert(ssa_var, false); // Arguments are typically not owned by the caller in a way that requires free
        if typ.is_resource() { self.survivors.insert(ssa_var, (name.clone(), false)); }
        self.args.push(crate::domain::entities::mir::MirArgument { name, typ, ssa_var });
    }

    pub fn new_ssa(&mut self) -> usize {
        let id = self.next_ssa;
        self.next_ssa += 1;
        id
    }

    pub fn define_variable(&mut self, name: &str, ssa_var: usize, typ: OnuType) {
        self.ssa_types.insert(ssa_var, typ.clone());
        // Default to not dynamic unless set_ssa_type specifies otherwise
        self.ssa_is_dynamic.entry(ssa_var).or_insert(false);
        
        if typ.is_resource() {
            let is_dyn = *self.ssa_is_dynamic.get(&ssa_var).unwrap_or(&false);
            self.survivors.insert(ssa_var, (name.to_string(), is_dyn));
        }
        
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), (ssa_var, typ));
        }
    }

    pub fn resolve_ssa_type(&self, ssa_var: usize) -> Option<OnuType> {
        self.ssa_types.get(&ssa_var).cloned()
    }

    pub fn set_ssa_type(&mut self, ssa_var: usize, typ: OnuType) {
        self.ssa_types.insert(ssa_var, typ.clone());
        if typ.is_resource() {
            let is_dyn = *self.ssa_is_dynamic.get(&ssa_var).unwrap_or(&false);
            self.survivors.insert(ssa_var, (format!("ssa_{}", ssa_var), is_dyn));
        }
    }

    pub fn set_ssa_is_dynamic(&mut self, ssa_var: usize, is_dynamic: bool) {
        self.ssa_is_dynamic.insert(ssa_var, is_dynamic);
        if let Some((name, _)) = self.survivors.get(&ssa_var) {
            let name = name.clone();
            self.survivors.insert(ssa_var, (name, is_dynamic));
        }
    }

    pub fn resolve_ssa_is_dynamic(&self, ssa_var: usize) -> bool {
        *self.ssa_is_dynamic.get(&ssa_var).unwrap_or(&false)
    }

    pub fn get_surviving_resources(&self) -> Vec<(usize, OnuType, String, bool)> {
        self.survivors.iter()
            .filter_map(|(id, (name, is_dyn))| {
                self.ssa_types.get(id).map(|t| (*id, t.clone(), name.clone(), *is_dyn))
            })
            .collect()
    }

    pub fn resolve_variable(&self, name: &str) -> Option<usize> {
        for scope in self.scopes.iter().rev() {
            if let Some((id, _)) = scope.get(name) {
                if self.consumed_vars.contains(id) {
                    return None;
                }
                return Some(*id);
            }
        }
        None
    }

    pub fn resolve_variable_type(&self, name: &str) -> Option<OnuType> {
        for scope in self.scopes.iter().rev() {
            if let Some((id, typ)) = scope.get(name) {
                if self.consumed_vars.contains(id) {
                    return None;
                }
                return Some(typ.clone());
            }
        }
        None
    }

    pub fn get_current_scope_variables(&self) -> Vec<(usize, OnuType)> {
        if let Some(scope) = self.scopes.last() {
            scope.values().cloned().filter(|(id, _)| !self.consumed_vars.contains(id)).collect()
        } else {
            Vec::new()
        }
    }

    pub fn mark_consumed(&mut self, ssa_var: usize) {
        self.survivors.remove(&ssa_var);
        self.consumed_vars.insert(ssa_var);
        // Clean up any pending drops for this variable since it's now owned by someone else
        self.pending_drops.retain(|(id, _, _, _)| *id != ssa_var);
    }

    pub fn get_consumed_vars(&self) -> std::collections::HashSet<usize> {
        self.consumed_vars.clone()
    }

    pub fn is_consumed(&self, ssa_var: usize) -> bool {
        self.consumed_vars.contains(&ssa_var)
    }

    pub fn set_consumed_vars(&mut self, consumed: std::collections::HashSet<usize>) {
        for id in &consumed {
            self.survivors.remove(id);
        }
        self.consumed_vars = consumed;
    }

    pub fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn exit_scope(&mut self) {
        self.scopes.pop();
    }

    pub fn schedule_drop(&mut self, ssa_var: usize, typ: OnuType) {
        if !self.pending_drops.iter().any(|(id, _, _, _)| *id == ssa_var) {
            let (name, is_dyn) = self.survivors.get(&ssa_var)
                .cloned()
                .unwrap_or_else(|| (format!("ssa_{}", ssa_var), *self.ssa_is_dynamic.get(&ssa_var).unwrap_or(&false)));
            self.pending_drops.push((ssa_var, typ, name, is_dyn));
        }
    }

    pub fn take_pending_drops(&mut self) -> Vec<(usize, OnuType, String, bool)> {
        std::mem::take(&mut self.pending_drops)
    }

    pub fn create_block(&mut self) -> usize {
        let id = self.blocks.len();
        self.blocks.push(BasicBlock {
            id,
            instructions: Vec::new(),
            terminator: MirTerminator::Unreachable,
        });
        id
    }

    pub fn switch_to_block(&mut self, id: usize) {
        if id < self.blocks.len() {
            self.current_block_idx = Some(id);
        }
    }

    pub fn clear_current_block(&mut self) {
        self.current_block_idx = None;
    }

    pub fn emit(&mut self, inst: MirInstruction) {
        if let Some(idx) = self.current_block_idx {
            self.blocks[idx].instructions.push(inst);
        }
    }

    pub fn build_index(&mut self, dest: usize, subject: crate::domain::entities::mir::MirOperand, index: usize) {
        self.emit(MirInstruction::Index { dest, subject, index });
    }

    pub fn build_alloc(&mut self, dest: usize, size_bytes: crate::domain::entities::mir::MirOperand) {
        self.emit(MirInstruction::Alloc { dest, size_bytes });
    }

    pub fn build_memcpy(&mut self, dest: crate::domain::entities::mir::MirOperand, src: crate::domain::entities::mir::MirOperand, size: crate::domain::entities::mir::MirOperand) {
        self.emit(MirInstruction::MemCopy { dest, src, size });
    }

    pub fn build_pointer_offset(&mut self, dest: usize, ptr: crate::domain::entities::mir::MirOperand, offset: crate::domain::entities::mir::MirOperand) {
        self.emit(MirInstruction::PointerOffset { dest, ptr, offset });
    }

    pub fn build_string_tuple(&mut self, dest: usize, len: crate::domain::entities::mir::MirOperand, ptr: crate::domain::entities::mir::MirOperand, is_dynamic: bool) {
        self.set_ssa_is_dynamic(dest, is_dynamic);
        self.emit(MirInstruction::Tuple {
            dest,
            elements: vec![
                len,
                ptr,
                crate::domain::entities::mir::MirOperand::Constant(crate::domain::entities::mir::MirLiteral::Boolean(is_dynamic)),
            ],
        });
    }

    pub fn build_binop(&mut self, dest: usize, op: crate::domain::entities::mir::MirBinOp, lhs: crate::domain::entities::mir::MirOperand, rhs: crate::domain::entities::mir::MirOperand) {
        self.emit(MirInstruction::BinaryOperation { dest, op, lhs, rhs });
    }

    pub fn build_assign(&mut self, dest: usize, src: crate::domain::entities::mir::MirOperand) {
        self.emit(MirInstruction::Assign { dest, src });
    }

    pub fn terminate(&mut self, term: MirTerminator) {
        if let Some(idx) = self.current_block_idx {
            self.blocks[idx].terminator = term;
        }
    }

    pub fn get_current_block_id(&self) -> Option<usize> {
        self.current_block_idx.map(|idx| self.blocks[idx].id)
    }

    pub fn build(self) -> MirFunction {
        MirFunction {
            name: self.name,
            args: self.args,
            return_type: self.return_type,
            blocks: self.blocks,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_variable_consumed() {
        let mut builder = MirBuilder::new("test".to_string(), OnuType::Nothing);
        builder.enter_scope();
        builder.define_variable("x", 10, OnuType::I64);
        
        assert_eq!(builder.resolve_variable("x"), Some(10));
        
        builder.mark_consumed(10);
        
        assert_eq!(builder.resolve_variable("x"), None, "resolve_variable should return None for consumed variables");
    }
}
