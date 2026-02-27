/// MIR Builder: Application Layer Helper
///
/// This struct encapsulates the state and logic for constructing a single MIR function.
/// It handles block management, SSA variable generation, and variable scoping.

use crate::domain::entities::mir::*;
use crate::domain::entities::types::OnuType;
use std::collections::HashMap;

pub struct MirBuilder {
    name: String,
    return_type: OnuType,
    blocks: Vec<BasicBlock>,
    current_block_idx: Option<usize>,
    next_ssa_var: usize,
    next_block_id: usize,
    scopes: Vec<HashMap<String, usize>>,
}

impl MirBuilder {
    pub fn new(name: String, return_type: OnuType) -> Self {
        Self {
            name,
            return_type,
            blocks: Vec::new(),
            current_block_idx: None,
            next_ssa_var: 0,
            next_block_id: 0,
            scopes: vec![HashMap::new()],
        }
    }

    pub fn build(self) -> MirFunction {
        MirFunction {
            name: self.name,
            args: Vec::new(),
            return_type: self.return_type,
            blocks: self.blocks,
        }
    }

    pub fn new_ssa(&mut self) -> usize {
        let var = self.next_ssa_var;
        self.next_ssa_var += 1;
        var
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    pub fn define_variable(&mut self, name: String, ssa_var: usize) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, ssa_var);
        }
    }

    pub fn resolve_variable(&self, name: &str) -> Option<usize> {
        for scope in self.scopes.iter().rev() {
            if let Some(var) = scope.get(name) {
                return Some(*var);
            }
        }
        None
    }

    pub fn create_block(&mut self) -> usize {
        let id = self.next_block_id;
        self.next_block_id += 1;
        self.blocks.push(BasicBlock {
            id,
            instructions: Vec::new(),
            terminator: MirTerminator::Unreachable,
        });
        id
    }

    pub fn switch_to_block(&mut self, id: usize) {
        if id == 9999 {
            self.current_block_idx = None;
            return;
        }
        self.current_block_idx = self.blocks.iter().position(|b| b.id == id);
    }

    pub fn emit(&mut self, inst: MirInstruction) {
        if let Some(idx) = self.current_block_idx {
            self.blocks[idx].instructions.push(inst);
        }
    }

    pub fn terminate(&mut self, term: MirTerminator) {
        if let Some(idx) = self.current_block_idx {
            self.blocks[idx].terminator = term;
        }
    }

    pub fn get_current_block_id(&self) -> Option<usize> {
        self.current_block_idx.map(|idx| self.blocks[idx].id)
    }
}
