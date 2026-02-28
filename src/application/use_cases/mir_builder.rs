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
    scopes: Vec<HashMap<String, usize>>,
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
        }
    }

    pub fn add_arg(&mut self, name: String, typ: OnuType, ssa_var: usize) {
        self.args.push(crate::domain::entities::mir::MirArgument { name, typ, ssa_var });
    }

    pub fn new_ssa(&mut self) -> usize {
        let id = self.next_ssa;
        self.next_ssa += 1;
        id
    }

    pub fn define_variable(&mut self, name: &str, ssa_var: usize) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), ssa_var);
        }
    }

    pub fn resolve_variable(&self, name: &str) -> Option<usize> {
        for scope in self.scopes.iter().rev() {
            if let Some(id) = scope.get(name) {
                return Some(*id);
            }
        }
        None
    }

    pub fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn exit_scope(&mut self) {
        self.scopes.pop();
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

    pub fn build(self) -> MirFunction {
        MirFunction {
            name: self.name,
            args: self.args,
            return_type: self.return_type,
            blocks: self.blocks,
        }
    }
}
