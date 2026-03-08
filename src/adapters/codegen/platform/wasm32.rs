/// WASM Direct Emitter: Platform Strategy for WebAssembly
///
/// Translates Ọ̀nụ MIR directly to a valid `.wasm` binary without LLVM.
/// Designed for the mobile-friendly offline web playground running on Netlify.
///
/// # Control Flow Strategy
///
/// Ọ̀nụ MIR uses basic blocks with arbitrary jumps. WebAssembly requires
/// structured control flow (block / loop / if). We use the **dispatch loop**
/// pattern: a loop with a `br_table` acting as a jump table keyed on a `$pc`
/// local variable.  This correctly handles all CFG shapes (TCO loops,
/// conditionals, linear chains) without a full relooper algorithm.
///
/// ```wasm
/// (loop $dispatch
///   (block $b_{N-1} (block $b_1 (block $b_0
///     local.get $pc
///     br_table 0 1 … N-1 (N-1)
///   )))       ;; exits land just after the respective block
///   ;; block 0 code: … terminator → set $pc, br (N-1) to $dispatch
///   ;; block 1 code: … terminator → set $pc, br (N-2) to $dispatch
///   …
/// )
/// unreachable
/// ```
use crate::domain::entities::error::OnuError;
use crate::domain::entities::mir::*;
use crate::domain::entities::types::OnuType;
use std::collections::HashMap;
use wasm_encoder::{
    BlockType, CodeSection, ConstExpr, DataSection, EntityType, ExportKind, ExportSection,
    Function, FunctionSection, GlobalSection, GlobalType, ImportSection, MemorySection,
    MemoryType, Module, TypeSection, ValType,
};

// ── Imported function indices (must match ImportSection order) ────────────────
const IMPORT_ONU_WRITE: u32 = 0; // onu_write(ptr: i32, len: i32) → void
const IMPORT_ONU_PRINT_I64: u32 = 1; // onu_print_i64(val: i64) → void
const NUM_IMPORTS: u32 = 2;

// ── Global variable indices ───────────────────────────────────────────────────
const GLOBAL_ARENA_PTR: u32 = 0; // arena bump pointer (mutable i32)

// ── Tuple (string struct) memory layout ──────────────────────────────────────
// Offset 0: len (i64, 8 bytes)
// Offset 8: ptr (i32, 4 bytes)
// Offset 12: is_dynamic flag (i32, 4 bytes)
const TUPLE_SIZE: u32 = 16;
const TUPLE_OFFSET_LEN: u32 = 0;
const TUPLE_OFFSET_PTR: u32 = 8;

/// Entry point: emit a complete `.wasm` binary from a MIR program.
pub struct WasmCodegenStrategy;

impl WasmCodegenStrategy {
    pub fn emit_program(program: &MirProgram) -> Result<Vec<u8>, OnuError> {
        WasmContext::new(program).build()
    }
}

// ── Internal context ──────────────────────────────────────────────────────────

struct WasmContext<'p> {
    program: &'p MirProgram,
    /// Maps function name → WASM function index (imports first, then user fns)
    fn_index: HashMap<String, u32>,
    /// Interned string literals: text → (data_section_offset, byte_length)
    string_map: HashMap<String, (u32, u32)>,
    /// Raw bytes for the data section
    string_data: Vec<u8>,
    /// Constant tables: table_name → (data_section_offset, values)
    table_map: HashMap<String, u32>,
    /// GlobalAlloc slots: name → offset in linear memory (after string data)
    global_alloc_map: HashMap<String, u32>,
    /// Byte offset where the arena starts (after all static allocations)
    arena_start: u32,
}

impl<'p> WasmContext<'p> {
    fn new(program: &'p MirProgram) -> Self {
        let mut ctx = WasmContext {
            program,
            fn_index: HashMap::new(),
            string_map: HashMap::new(),
            string_data: Vec::new(),
            table_map: HashMap::new(),
            global_alloc_map: HashMap::new(),
            arena_start: 0,
        };
        ctx.collect_statics();
        ctx
    }

    /// First pass: collect all compile-time static data (strings, tables,
    /// global-alloc blobs) so we know their offsets before emitting code.
    fn collect_statics(&mut self) {
        for func in &self.program.functions {
            for block in &func.blocks {
                for inst in &block.instructions {
                    self.collect_from_inst(inst);
                }
            }
        }
        // arena_start = byte after all static data, aligned to 8 bytes
        let raw = self.string_data.len() as u32;
        self.arena_start = (raw + 7) & !7; // round up to 8-byte alignment
        // Pad string data to arena_start
        self.string_data.resize(self.arena_start as usize, 0);
    }

    fn collect_from_inst(&mut self, inst: &MirInstruction) {
        match inst {
            MirInstruction::Emit(op) => {
                if let MirOperand::Constant(MirLiteral::Text(s)) = op {
                    self.intern_string(s);
                }
            }
            MirInstruction::ConstantTableLoad { name, values, .. } => {
                if !self.table_map.contains_key(name) {
                    let offset = self.string_data.len() as u32;
                    self.table_map.insert(name.clone(), offset);
                    for v in values {
                        self.string_data.extend_from_slice(&v.to_le_bytes());
                    }
                }
            }
            MirInstruction::GlobalAlloc {
                name, size_bytes, ..
            } => {
                if !self.global_alloc_map.contains_key(name) {
                    let offset = self.string_data.len() as u32;
                    self.global_alloc_map.insert(name.clone(), offset);
                    // Reserve size_bytes (zero-initialised by default in WASM)
                    let new_len = self.string_data.len() + size_bytes;
                    self.string_data.resize(new_len, 0);
                }
            }
            _ => {}
        }
    }

    fn intern_string(&mut self, s: &str) -> (u32, u32) {
        if let Some(&pair) = self.string_map.get(s) {
            return pair;
        }
        let offset = self.string_data.len() as u32;
        let bytes = s.as_bytes();
        let len = bytes.len() as u32;
        self.string_data.extend_from_slice(bytes);
        self.string_map.insert(s.to_string(), (offset, len));
        (offset, len)
    }

    fn build(mut self) -> Result<Vec<u8>, OnuError> {
        let mut module = Module::new();

        // ── 1. Types ─────────────────────────────────────────────────────────
        let mut types = TypeSection::new();

        // Import sigs (must be first, so indices match IMPORT_* constants)
        // Type 0: onu_write(i32, i32) → []
        types.ty().function([ValType::I32, ValType::I32], []);
        // Type 1: onu_print_i64(i64) → []
        types.ty().function([ValType::I64], []);

        // User function sigs: collect (param types, result types) and deduplicate
        let mut sig_map: HashMap<(Vec<ValType>, Vec<ValType>), u32> = HashMap::new();
        let mut next_type_idx = 2u32; // 0,1 reserved for imports

        for func in &self.program.functions {
            let (params, results) = fn_sig(func);
            let key = (params.clone(), results.clone());
            if !sig_map.contains_key(&key) {
                types
                    .ty()
                    .function(params.iter().copied(), results.iter().copied());
                sig_map.insert(key, next_type_idx);
                next_type_idx += 1;
            }
        }
        module.section(&types);

        // ── 2. Imports ───────────────────────────────────────────────────────
        let mut imports = ImportSection::new();
        imports.import("env", "onu_write", EntityType::Function(0));
        imports.import("env", "onu_print_i64", EntityType::Function(1));
        module.section(&imports);

        // ── 3. Function section (user functions) ─────────────────────────────
        let mut functions = FunctionSection::new();
        for (fi, func) in self.program.functions.iter().enumerate() {
            let wasm_idx = NUM_IMPORTS + fi as u32;
            self.fn_index.insert(func.name.clone(), wasm_idx);
            let (params, results) = fn_sig(func);
            let key = (params, results);
            functions.function(*sig_map.get(&key).unwrap());
        }
        module.section(&functions);

        // ── 4. Memory ────────────────────────────────────────────────────────
        // We request enough initial pages to cover static data + 1 MB arena.
        let static_bytes = self.string_data.len() as u64;
        let arena_mb = 2u64 * 1024 * 1024; // 2 MiB for arena
        let wasm_page = 65536u64;
        let min_pages = (static_bytes + arena_mb + wasm_page - 1) / wasm_page;
        let min_pages = min_pages.max(2);

        let mut memories = MemorySection::new();
        memories.memory(MemoryType {
            minimum: min_pages,
            maximum: Some(256), // cap at 16 MiB
            memory64: false,
            shared: false,
            page_size_log2: None,
        });
        module.section(&memories);

        // ── 5. Globals ───────────────────────────────────────────────────────
        let mut globals = GlobalSection::new();
        // Global 0: $arena_ptr — mutable i32, initialised to first free byte
        globals.global(
            GlobalType {
                val_type: ValType::I32,
                mutable: true,
                shared: false,
            },
            &ConstExpr::i32_const(self.arena_start as i32),
        );
        module.section(&globals);

        // ── 6. Exports ───────────────────────────────────────────────────────
        let mut exports = ExportSection::new();
        exports.export("memory", ExportKind::Memory, 0);
        // Export every function; "run" is the canonical entry point
        for func in &self.program.functions {
            let idx = *self.fn_index.get(&func.name).unwrap();
            exports.export(&func.name, ExportKind::Func, idx);
        }
        module.section(&exports);

        // ── 7. Data ──────────────────────────────────────────────────────────
        if !self.string_data.is_empty() {
            let mut data = DataSection::new();
            data.active(
                0, // memory index
                &ConstExpr::i32_const(0),
                self.string_data.clone(),
            );
            module.section(&data);
        }

        // ── 8. Code ──────────────────────────────────────────────────────────
        let mut codes = CodeSection::new();
        // We need fn_index to be fully populated before emitting code
        for func in self.program.functions.iter() {
            let f = self.emit_function(func)?;
            codes.function(&f);
        }
        module.section(&codes);

        Ok(module.finish())
    }

    // ── Function body emission ────────────────────────────────────────────────

    fn emit_function(&self, func: &MirFunction) -> Result<Function, OnuError> {
        let ssa_types = collect_ssa_types(func);
        let local_map = LocalMap::new(func, &ssa_types);

        // Declare extra locals: $__pc (i32) + one per non-arg SSA var
        let extra_locals: Vec<(u32, ValType)> = local_map
            .extra_types
            .iter()
            .map(|&vt| (1u32, vt))
            .collect();

        let mut f = Function::new(extra_locals);
        let mut insns = f.instructions();

        let n = func.blocks.len();

        // Initialise $pc = 0
        insns.i32_const(0).local_set(local_map.pc_local);

        // Outer dispatch loop
        insns.loop_(BlockType::Empty);

        // N nested blocks for the jump table
        for _ in 0..n {
            insns.block(BlockType::Empty);
        }

        // Jump table dispatch
        let targets: Vec<u32> = (0..n as u32).collect();
        insns.local_get(local_map.pc_local);
        insns.br_table(targets, (n as u32).saturating_sub(1));
        insns.unreachable(); // dead code after br_table

        // Emit each basic block
        for k in 0..n {
            insns.end(); // close the (N-k)th nested block → block k code starts

            let block = &func.blocks[k];
            let dispatch_depth = (n - 1 - k) as u32;

            // Emit instructions
            for inst in &block.instructions {
                self.emit_instruction(&mut insns, inst, &local_map, &ssa_types)?;
            }

            // Emit terminator
            match &block.terminator {
                MirTerminator::Return(op) => {
                    if !matches!(op, MirOperand::Constant(MirLiteral::Nothing))
                        && !matches!(func.return_type, OnuType::Nothing)
                    {
                        self.emit_operand(&mut insns, op, &local_map, &ssa_types);
                    }
                    insns.return_();
                }
                MirTerminator::Branch(target) => {
                    insns.i32_const(*target as i32);
                    insns.local_set(local_map.pc_local);
                    insns.br(dispatch_depth);
                }
                MirTerminator::CondBranch {
                    condition,
                    then_block,
                    else_block,
                } => {
                    // Push condition (i32)
                    self.emit_operand_as_i32(&mut insns, condition, &local_map, &ssa_types);
                    insns.if_(BlockType::Empty);
                    insns.i32_const(*then_block as i32);
                    insns.local_set(local_map.pc_local);
                    insns.else_();
                    insns.i32_const(*else_block as i32);
                    insns.local_set(local_map.pc_local);
                    insns.end(); // close if
                    insns.br(dispatch_depth);
                }
                MirTerminator::Unreachable => {
                    insns.unreachable();
                }
            }
        }

        // Close dispatch loop
        insns.end();
        // After loop (unreachable — every path returns or loops)
        insns.unreachable();
        // Close function
        insns.end();

        Ok(f)
    }

    // ── Instruction emission ──────────────────────────────────────────────────

    fn emit_instruction(
        &self,
        insns: &mut wasm_encoder::InstructionSink<'_>,
        inst: &MirInstruction,
        lm: &LocalMap,
        ssa_types: &HashMap<usize, OnuType>,
    ) -> Result<(), OnuError> {
        use MirInstruction::*;
        match inst {
            // ── Assign ──────────────────────────────────────────────────────
            Assign { dest, src } => {
                self.emit_operand(insns, src, lm, ssa_types);
                insns.local_set(lm.local_idx(*dest));
            }

            // ── BinaryOperation ─────────────────────────────────────────────
            BinaryOperation {
                dest,
                op,
                lhs,
                rhs,
                dest_type,
            } => {
                self.emit_binop(insns, *dest, op, lhs, rhs, dest_type, lm, ssa_types);
            }

            // ── Call ────────────────────────────────────────────────────────
            Call {
                dest,
                name,
                args,
                return_type,
                ..
            } => {
                for arg in args {
                    self.emit_operand(insns, arg, lm, ssa_types);
                }
                if let Some(&fidx) = self.fn_index.get(name) {
                    insns.call(fidx);
                } else {
                    // Unknown external call → push a zero of the return type
                    match return_type {
                        OnuType::Nothing => {}
                        OnuType::Boolean => {
                            insns.i32_const(0);
                        }
                        OnuType::F64 => {
                            insns.f64_const(wasm_encoder::Ieee64::from(0.0_f64));
                        }
                        _ => {
                            insns.i64_const(0);
                        }
                    }
                }
                if !matches!(return_type, OnuType::Nothing) {
                    insns.local_set(lm.local_idx(*dest));
                }
            }

            // ── Emit ────────────────────────────────────────────────────────
            Emit(op) => {
                match op {
                    MirOperand::Constant(MirLiteral::Text(s)) => {
                        // Write string bytes + newline
                        let (offset, len) = self
                            .string_map
                            .get(s.as_str())
                            .copied()
                            .unwrap_or((0, 0));
                        insns.i32_const(offset as i32);
                        insns.i32_const(len as i32);
                        insns.call(IMPORT_ONU_WRITE);
                        // Trailing newline: write a single '\n' byte
                        // (stored in a known byte in memory or just emit '\n' = 0x0A)
                        // We skip newline for WASM since JS handles display
                    }
                    MirOperand::Variable(ssa, _) => {
                        let typ = ssa_types
                            .get(ssa)
                            .cloned()
                            .unwrap_or(OnuType::I64);
                        match &typ {
                            OnuType::Strings => {
                                // String struct pointer: load ptr and len, call onu_write
                                insns.local_get(lm.local_idx(*ssa)); // base ptr (i32)
                                insns.i32_const(TUPLE_OFFSET_PTR as i32);
                                insns.i32_add();
                                insns.i32_load(wasm_encoder::MemArg {
                                    offset: 0u64,
                                    align: 2,
                                    memory_index: 0,
                                }); // ptr (i32)
                                insns.local_get(lm.local_idx(*ssa)); // base ptr again
                                insns.i64_load(wasm_encoder::MemArg {
                                    offset: TUPLE_OFFSET_LEN as u64,
                                    align: 3,
                                    memory_index: 0,
                                }); // len (i64)
                                insns.i32_wrap_i64(); // truncate to i32
                                // stack: [ptr, len] — onu_write(ptr, len)
                                // Reorder: need [ptr, len] but we have [ptr, len] already
                                insns.call(IMPORT_ONU_WRITE);
                            }
                            _ => {
                                // Integer value: use onu_print_i64
                                insns.local_get(lm.local_idx(*ssa));
                                // If not already i64, extend
                                if mir_to_wasm_type(&typ) == ValType::I32 {
                                    insns.i64_extend_i32_s();
                                }
                                insns.call(IMPORT_ONU_PRINT_I64);
                            }
                        }
                    }
                    _ => {} // Nothing/boolean constants: no output
                }
            }

            // ── Drop ────────────────────────────────────────────────────────
            Drop { .. } => {} // zero-cost in arena model

            // ── Alloc ───────────────────────────────────────────────────────
            Alloc { dest, size_bytes } => {
                // Bump allocator: ptr = arena_ptr; arena_ptr += size_bytes
                insns.global_get(GLOBAL_ARENA_PTR);
                insns.local_set(lm.local_idx(*dest)); // save old ptr in dest

                // Compute new arena ptr
                insns.global_get(GLOBAL_ARENA_PTR);
                self.emit_operand(insns, size_bytes, lm, ssa_types);
                // size_bytes might be i64; truncate to i32 for ptr arithmetic
                if operand_is_i64(size_bytes, ssa_types) {
                    insns.i32_wrap_i64();
                }
                insns.i32_add();
                insns.global_set(GLOBAL_ARENA_PTR);
            }

            // ── StackAlloc ──────────────────────────────────────────────────
            StackAlloc { dest, size_bytes } => {
                // Treat as Alloc in WASM (no true alloca)
                insns.global_get(GLOBAL_ARENA_PTR);
                insns.local_set(lm.local_idx(*dest));

                insns.global_get(GLOBAL_ARENA_PTR);
                insns.i32_const(*size_bytes as i32);
                insns.i32_add();
                insns.global_set(GLOBAL_ARENA_PTR);
            }

            // ── GlobalAlloc ─────────────────────────────────────────────────
            GlobalAlloc { dest, name, .. } => {
                let offset = self
                    .global_alloc_map
                    .get(name)
                    .copied()
                    .unwrap_or(self.arena_start);
                insns.i32_const(offset as i32);
                insns.local_set(lm.local_idx(*dest));
            }

            // ── SaveArena / RestoreArena ─────────────────────────────────────
            SaveArena { dest } => {
                insns.global_get(GLOBAL_ARENA_PTR);
                insns.local_set(lm.local_idx(*dest));
            }
            RestoreArena { saved } => {
                self.emit_operand(insns, saved, lm, ssa_types);
                // saved is i32 (ptr type)
                if operand_is_i64(saved, ssa_types) {
                    insns.i32_wrap_i64();
                }
                insns.global_set(GLOBAL_ARENA_PTR);
            }

            // ── PointerOffset ────────────────────────────────────────────────
            PointerOffset { dest, ptr, offset } => {
                self.emit_operand_as_i32(insns, ptr, lm, ssa_types);
                self.emit_operand_as_i32(insns, offset, lm, ssa_types);
                insns.i32_add();
                insns.local_set(lm.local_idx(*dest));
            }

            // ── Load ─────────────────────────────────────────────────────────
            Load { dest, ptr, typ } => {
                self.emit_operand_as_i32(insns, ptr, lm, ssa_types);
                match typ {
                    OnuType::I64 | OnuType::U64 => {
                        insns.i64_load(wasm_encoder::MemArg {
                            offset: 0u64,
                            align: 3,
                            memory_index: 0,
                        });
                    }
                    OnuType::Boolean | OnuType::Ptr => {
                        insns.i32_load(wasm_encoder::MemArg {
                            offset: 0u64,
                            align: 2,
                            memory_index: 0,
                        });
                    }
                    _ => {
                        insns.i64_load(wasm_encoder::MemArg {
                            offset: 0u64,
                            align: 3,
                            memory_index: 0,
                        });
                    }
                }
                insns.local_set(lm.local_idx(*dest));
            }

            // ── Store ────────────────────────────────────────────────────────
            Store { ptr, value } => {
                self.emit_operand_as_i32(insns, ptr, lm, ssa_types);
                self.emit_operand(insns, value, lm, ssa_types);
                if operand_is_i64(value, ssa_types) {
                    insns.i64_store(wasm_encoder::MemArg {
                        offset: 0u64,
                        align: 3,
                        memory_index: 0,
                    });
                } else {
                    insns.i32_store(wasm_encoder::MemArg {
                        offset: 0u64,
                        align: 2,
                        memory_index: 0,
                    });
                }
            }

            // ── TypedStore ───────────────────────────────────────────────────
            TypedStore { ptr, value, typ } => {
                self.emit_operand_as_i32(insns, ptr, lm, ssa_types);
                self.emit_operand(insns, value, lm, ssa_types);
                match typ {
                    OnuType::I64 | OnuType::U64 => {
                        insns.i64_store(wasm_encoder::MemArg {
                            offset: 0u64,
                            align: 3,
                            memory_index: 0,
                        });
                    }
                    OnuType::Boolean | OnuType::Ptr | OnuType::Strings | OnuType::Nothing => {
                        if operand_is_i64(value, ssa_types) {
                            insns.i32_wrap_i64();
                        }
                        insns.i32_store(wasm_encoder::MemArg {
                            offset: 0u64,
                            align: 2,
                            memory_index: 0,
                        });
                    }
                    _ => {
                        insns.i64_store(wasm_encoder::MemArg {
                            offset: 0u64,
                            align: 3,
                            memory_index: 0,
                        });
                    }
                }
            }

            // ── MemCopy ──────────────────────────────────────────────────────
            MemCopy { dest, src, size } => {
                self.emit_operand_as_i32(insns, dest, lm, ssa_types);
                self.emit_operand_as_i32(insns, src, lm, ssa_types);
                self.emit_operand_as_i32(insns, size, lm, ssa_types);
                insns.memory_copy(0, 0);
            }

            // ── MemSet ───────────────────────────────────────────────────────
            MemSet { ptr, value, size } => {
                self.emit_operand_as_i32(insns, ptr, lm, ssa_types);
                self.emit_operand_as_i32(insns, value, lm, ssa_types);
                self.emit_operand_as_i32(insns, size, lm, ssa_types);
                insns.memory_fill(0);
            }

            // ── Promote ──────────────────────────────────────────────────────
            Promote { dest, src, to_type } => {
                self.emit_operand(insns, src, lm, ssa_types);
                let from_wt = operand_wasm_type(src, ssa_types);
                let to_wt = mir_to_wasm_type(to_type);
                match (from_wt, to_wt) {
                    (ValType::I32, ValType::I64) => {
                        insns.i64_extend_i32_s();
                    }
                    (ValType::I64, ValType::I32) => {
                        insns.i32_wrap_i64();
                    }
                    _ => {} // same type or f64
                }
                insns.local_set(lm.local_idx(*dest));
            }

            // ── BitCast ──────────────────────────────────────────────────────
            BitCast { dest, src, .. } => {
                // Reinterpret: in WASM reinterpret ops only work between int↔float
                // For int→int, just copy (same bit width assumed)
                self.emit_operand(insns, src, lm, ssa_types);
                insns.local_set(lm.local_idx(*dest));
            }

            // ── Tuple ────────────────────────────────────────────────────────
            Tuple { dest, elements } => {
                // Allocate TUPLE_SIZE bytes in the arena, store elements
                // dest holds an i32 pointer
                insns.global_get(GLOBAL_ARENA_PTR);
                insns.local_set(lm.local_idx(*dest)); // save ptr

                // Write element 0 at offset 0 (len, i64, 8 bytes)
                if let Some(e0) = elements.first() {
                    insns.local_get(lm.local_idx(*dest));
                    self.emit_operand(insns, e0, lm, ssa_types);
                    // Force to i64
                    if operand_wasm_type(e0, ssa_types) == ValType::I32 {
                        insns.i64_extend_i32_s();
                    }
                    insns.i64_store(wasm_encoder::MemArg {
                        offset: TUPLE_OFFSET_LEN as u64,
                        align: 3,
                        memory_index: 0,
                    });
                }
                // Write element 1 at offset 8 (ptr, i32)
                if let Some(e1) = elements.get(1) {
                    insns.local_get(lm.local_idx(*dest));
                    self.emit_operand(insns, e1, lm, ssa_types);
                    if operand_wasm_type(e1, ssa_types) == ValType::I64 {
                        insns.i32_wrap_i64();
                    }
                    insns.i32_store(wasm_encoder::MemArg {
                        offset: TUPLE_OFFSET_PTR as u64,
                        align: 2,
                        memory_index: 0,
                    });
                }
                // Write element 2 at offset 12 (is_dynamic, i32)
                if let Some(e2) = elements.get(2) {
                    insns.local_get(lm.local_idx(*dest));
                    self.emit_operand(insns, e2, lm, ssa_types);
                    if operand_wasm_type(e2, ssa_types) == ValType::I64 {
                        insns.i32_wrap_i64();
                    }
                    insns.i32_store(wasm_encoder::MemArg {
                        offset: 12u64,
                        align: 2,
                        memory_index: 0,
                    });
                }

                // Bump arena pointer by TUPLE_SIZE
                insns.global_get(GLOBAL_ARENA_PTR);
                insns.i32_const(TUPLE_SIZE as i32);
                insns.i32_add();
                insns.global_set(GLOBAL_ARENA_PTR);
            }

            // ── Index ────────────────────────────────────────────────────────
            Index { dest, subject, index } => {
                // Load field `index` from the tuple struct
                self.emit_operand_as_i32(insns, subject, lm, ssa_types);
                let dest_vt = mir_to_wasm_type(
                    ssa_types.get(dest).unwrap_or(&OnuType::I64),
                );
                match index {
                    0 => {
                        // len (i64)
                        insns.i64_load(wasm_encoder::MemArg {
                            offset: TUPLE_OFFSET_LEN as u64,
                            align: 3,
                            memory_index: 0,
                        });
                        if dest_vt == ValType::I32 {
                            insns.i32_wrap_i64();
                        }
                    }
                    1 => {
                        // ptr (i32)
                        insns.i32_load(wasm_encoder::MemArg {
                            offset: TUPLE_OFFSET_PTR as u64,
                            align: 2,
                            memory_index: 0,
                        });
                        if dest_vt == ValType::I64 {
                            insns.i64_extend_i32_s();
                        }
                    }
                    _ => {
                        // is_dynamic or other fields
                        insns.i32_load(wasm_encoder::MemArg {
                            offset: 12u64,
                            align: 2,
                            memory_index: 0,
                        });
                        if dest_vt == ValType::I64 {
                            insns.i64_extend_i32_s();
                        }
                    }
                }
                insns.local_set(lm.local_idx(*dest));
            }

            // ── ConstantTableLoad ────────────────────────────────────────────
            ConstantTableLoad {
                dest,
                name,
                index,
                ..
            } => {
                let base_offset = self.table_map.get(name).copied().unwrap_or(0);
                // address = base_offset + index * 8
                insns.i32_const(base_offset as i32);
                self.emit_operand_as_i32(insns, index, lm, ssa_types);
                insns.i32_const(8);
                insns.i32_mul();
                insns.i32_add();
                insns.i64_load(wasm_encoder::MemArg {
                    offset: 0u64,
                    align: 3,
                    memory_index: 0,
                });
                insns.local_set(lm.local_idx(*dest));
            }

            // ── FunnelShiftRight ─────────────────────────────────────────────
            FunnelShiftRight {
                dest,
                hi,
                lo,
                amount,
                width,
            } => {
                // Emulate: fshr(hi, lo, amount)
                // For rotr-32: (hi | lo) >> amount | (hi | lo) << (32 - amount)
                // hi == lo in the rotation case
                match width {
                    32 => {
                        // Truncate to i32, rotate, extend
                        self.emit_operand(insns, hi, lm, ssa_types);
                        if operand_wasm_type(hi, ssa_types) == ValType::I64 {
                            insns.i32_wrap_i64();
                        }
                        self.emit_operand(insns, amount, lm, ssa_types);
                        if operand_wasm_type(amount, ssa_types) == ValType::I64 {
                            insns.i32_wrap_i64();
                        }
                        insns.i32_rotr();
                        insns.i64_extend_i32_u();
                    }
                    64 => {
                        self.emit_operand(insns, hi, lm, ssa_types);
                        if operand_wasm_type(hi, ssa_types) == ValType::I32 {
                            insns.i64_extend_i32_u();
                        }
                        self.emit_operand(insns, lo, lm, ssa_types);
                        if operand_wasm_type(lo, ssa_types) == ValType::I32 {
                            insns.i64_extend_i32_u();
                        }
                        let _ = lo; // suppress unused-variable lint
                        self.emit_operand(insns, amount, lm, ssa_types);
                        if operand_wasm_type(amount, ssa_types) == ValType::I32 {
                            insns.i64_extend_i32_u();
                        }
                        insns.i64_rotr();
                    }
                    _ => {
                        // Fallback: manual shift+or for arbitrary width
                        // dst = (hi >> amount) | (lo << (width - amount))
                        self.emit_operand(insns, hi, lm, ssa_types);
                        if operand_wasm_type(hi, ssa_types) == ValType::I32 {
                            insns.i64_extend_i32_u();
                        }
                        self.emit_operand(insns, amount, lm, ssa_types);
                        if operand_wasm_type(amount, ssa_types) == ValType::I32 {
                            insns.i64_extend_i32_u();
                        }
                        insns.i64_shr_u();

                        self.emit_operand(insns, lo, lm, ssa_types);
                        if operand_wasm_type(lo, ssa_types) == ValType::I32 {
                            insns.i64_extend_i32_u();
                        }
                        insns.i64_const(*width as i64);
                        self.emit_operand(insns, amount, lm, ssa_types);
                        if operand_wasm_type(amount, ssa_types) == ValType::I32 {
                            insns.i64_extend_i32_u();
                        }
                        insns.i64_sub();
                        insns.i64_shl();
                        insns.i64_or();
                    }
                }
                insns.local_set(lm.local_idx(*dest));
            }

            // ── BufferedWrite ────────────────────────────────────────────────
            BufferedWrite { ptr, len } => {
                self.emit_operand_as_i32(insns, ptr, lm, ssa_types);
                self.emit_operand_as_i32(insns, len, lm, ssa_types);
                insns.call(IMPORT_ONU_WRITE);
            }

            // ── FlushStdout ──────────────────────────────────────────────────
            FlushStdout => {} // No-op: onu_write is unbuffered in WASM
        }
        Ok(())
    }

    // ── Operand helpers ───────────────────────────────────────────────────────

    fn emit_operand(
        &self,
        insns: &mut wasm_encoder::InstructionSink<'_>,
        op: &MirOperand,
        lm: &LocalMap,
        ssa_types: &HashMap<usize, OnuType>,
    ) {
        match op {
            MirOperand::Constant(lit) => self.emit_literal(insns, lit),
            MirOperand::Variable(ssa, _) => {
                insns.local_get(lm.local_idx(*ssa));
            }
        }
    }

    /// Like `emit_operand` but guarantees an i32 on the stack.
    fn emit_operand_as_i32(
        &self,
        insns: &mut wasm_encoder::InstructionSink<'_>,
        op: &MirOperand,
        lm: &LocalMap,
        ssa_types: &HashMap<usize, OnuType>,
    ) {
        self.emit_operand(insns, op, lm, ssa_types);
        if operand_is_i64(op, ssa_types) {
            insns.i32_wrap_i64();
        }
    }

    /// Ensures an i32 for branch conditions (booleans from i64 comparisons).
    fn emit_operand_as_i32_cond(
        &self,
        insns: &mut wasm_encoder::InstructionSink<'_>,
        op: &MirOperand,
        lm: &LocalMap,
        ssa_types: &HashMap<usize, OnuType>,
    ) {
        self.emit_operand_as_i32(insns, op, lm, ssa_types);
    }

    fn emit_literal(&self, insns: &mut wasm_encoder::InstructionSink<'_>, lit: &MirLiteral) {
        match lit {
            MirLiteral::I64(v) => {
                insns.i64_const(*v);
            }
            MirLiteral::F64(bits) => {
                insns.f64_const(wasm_encoder::Ieee64::new(*bits));
            }
            MirLiteral::Boolean(b) => {
                insns.i32_const(if *b { 1 } else { 0 });
            }
            MirLiteral::Text(s) => {
                // Text literals are represented as a pointer (i32) to the data section.
                // We return the offset of the string.
                let (offset, _) = self.string_map.get(s.as_str()).copied().unwrap_or((0, 0));
                insns.i32_const(offset as i32);
            }
            MirLiteral::Nothing => {
                insns.i32_const(0); // null ptr / unit
            }
            MirLiteral::WideInt(s, _bits) => {
                // Best-effort: parse as i64 (truncated)
                let v: i64 = s.parse().unwrap_or(0);
                insns.i64_const(v);
            }
        }
    }

    // ── Binary operation emission ─────────────────────────────────────────────

    fn emit_binop(
        &self,
        insns: &mut wasm_encoder::InstructionSink<'_>,
        dest: usize,
        op: &MirBinOp,
        lhs: &MirOperand,
        rhs: &MirOperand,
        dest_type: &OnuType,
        lm: &LocalMap,
        ssa_types: &HashMap<usize, OnuType>,
    ) {
        let lhs_i64 = operand_is_i64(lhs, ssa_types);
        let rhs_i64 = operand_is_i64(rhs, ssa_types);
        // Use i64 ops when either operand is i64, otherwise use i32
        let use_i64 = lhs_i64 || rhs_i64;

        self.emit_operand(insns, lhs, lm, ssa_types);
        if use_i64 && !lhs_i64 {
            insns.i64_extend_i32_s();
        } else if !use_i64 && lhs_i64 {
            insns.i32_wrap_i64();
        }

        self.emit_operand(insns, rhs, lm, ssa_types);
        if use_i64 && !rhs_i64 {
            insns.i64_extend_i32_s();
        } else if !use_i64 && rhs_i64 {
            insns.i32_wrap_i64();
        }

        if use_i64 {
            match op {
                MirBinOp::Add => {
                    insns.i64_add();
                }
                MirBinOp::Sub => {
                    insns.i64_sub();
                }
                MirBinOp::Mul => {
                    insns.i64_mul();
                }
                MirBinOp::Div => {
                    insns.i64_div_s();
                }
                MirBinOp::Eq => {
                    insns.i64_eq();
                } // → i32
                MirBinOp::Ne => {
                    insns.i64_ne();
                } // → i32
                MirBinOp::Gt => {
                    insns.i64_gt_s();
                } // → i32
                MirBinOp::Lt => {
                    insns.i64_lt_s();
                } // → i32
                MirBinOp::And => {
                    insns.i64_and();
                }
                MirBinOp::Or => {
                    insns.i64_or();
                }
                MirBinOp::Xor => {
                    insns.i64_xor();
                }
                MirBinOp::Shr => {
                    insns.i64_shr_u();
                }
                MirBinOp::Shl => {
                    insns.i64_shl();
                }
            }
        } else {
            match op {
                MirBinOp::Add => {
                    insns.i32_add();
                }
                MirBinOp::Sub => {
                    insns.i32_sub();
                }
                MirBinOp::Mul => {
                    insns.i32_mul();
                }
                MirBinOp::Div => {
                    insns.i32_div_s();
                }
                MirBinOp::Eq => {
                    insns.i32_eq();
                }
                MirBinOp::Ne => {
                    insns.i32_ne();
                }
                MirBinOp::Gt => {
                    insns.i32_gt_s();
                }
                MirBinOp::Lt => {
                    insns.i32_lt_s();
                }
                MirBinOp::And => {
                    insns.i32_and();
                }
                MirBinOp::Or => {
                    insns.i32_or();
                }
                MirBinOp::Xor => {
                    insns.i32_xor();
                }
                MirBinOp::Shr => {
                    insns.i32_shr_u();
                }
                MirBinOp::Shl => {
                    insns.i32_shl();
                }
            }
        }

        // Comparison ops produce i32 even when operands were i64.
        // If dest is Boolean → i32 local, that's fine.
        // If dest is I64 and we have an i32 comparison result, extend.
        let dest_wt = mir_to_wasm_type(dest_type);
        let result_wt = match op {
            MirBinOp::Eq | MirBinOp::Ne | MirBinOp::Gt | MirBinOp::Lt => ValType::I32,
            _ => {
                if use_i64 {
                    ValType::I64
                } else {
                    ValType::I32
                }
            }
        };

        if dest_wt == ValType::I64 && result_wt == ValType::I32 {
            insns.i64_extend_i32_s();
        } else if dest_wt == ValType::I32 && result_wt == ValType::I64 {
            insns.i32_wrap_i64();
        }

        insns.local_set(lm.local_idx(dest));
    }
}

// ── Type helpers ──────────────────────────────────────────────────────────────

pub fn mir_to_wasm_type(t: &OnuType) -> ValType {
    match t {
        OnuType::Boolean | OnuType::Ptr | OnuType::Strings | OnuType::Nothing | OnuType::Matrix => {
            ValType::I32
        }
        OnuType::F64 | OnuType::F32 => ValType::F64,
        _ => ValType::I64, // all integer types
    }
}

fn operand_wasm_type(op: &MirOperand, ssa_types: &HashMap<usize, OnuType>) -> ValType {
    match op {
        MirOperand::Constant(lit) => match lit {
            MirLiteral::I64(_) | MirLiteral::WideInt(_, _) => ValType::I64,
            MirLiteral::F64(_) => ValType::F64,
            MirLiteral::Boolean(_) | MirLiteral::Text(_) | MirLiteral::Nothing => ValType::I32,
        },
        MirOperand::Variable(ssa, _) => {
            mir_to_wasm_type(ssa_types.get(ssa).unwrap_or(&OnuType::I64))
        }
    }
}

fn operand_is_i64(op: &MirOperand, ssa_types: &HashMap<usize, OnuType>) -> bool {
    operand_wasm_type(op, ssa_types) == ValType::I64
}

// ── Local map ─────────────────────────────────────────────────────────────────

struct LocalMap {
    /// Maps SSA var → WASM local index
    ssa_to_local: HashMap<usize, u32>,
    /// Index of the dispatch `$pc` local
    pc_local: u32,
    /// ValType for each extra local (in order; first entry = $pc = i32)
    extra_types: Vec<ValType>,
}

impl LocalMap {
    fn new(func: &MirFunction, ssa_types: &HashMap<usize, OnuType>) -> Self {
        let mut ssa_to_local: HashMap<usize, u32> = HashMap::new();

        // WASM function parameters are locals 0..n_args-1
        for (i, arg) in func.args.iter().enumerate() {
            ssa_to_local.insert(arg.ssa_var, i as u32);
        }

        let pc_local = func.args.len() as u32;
        let mut extra_types: Vec<ValType> = vec![ValType::I32]; // $__pc
        let mut next_local = pc_local + 1;

        // Collect all SSA destinations not already mapped (non-arg vars)
        let mut dests: Vec<usize> = ssa_types
            .keys()
            .copied()
            .filter(|k| !ssa_to_local.contains_key(k))
            .collect();
        dests.sort_unstable();

        for ssa in dests {
            ssa_to_local.insert(ssa, next_local);
            let vt = mir_to_wasm_type(ssa_types.get(&ssa).unwrap_or(&OnuType::I64));
            extra_types.push(vt);
            next_local += 1;
        }

        LocalMap {
            ssa_to_local,
            pc_local,
            extra_types,
        }
    }

    fn local_idx(&self, ssa: usize) -> u32 {
        *self.ssa_to_local.get(&ssa).unwrap_or_else(|| {
            // Should not happen in well-formed MIR; return a safe sentinel
            &0
        })
    }
}

// ── SSA type inference ────────────────────────────────────────────────────────

fn collect_ssa_types(func: &MirFunction) -> HashMap<usize, OnuType> {
    let mut types: HashMap<usize, OnuType> = HashMap::new();

    for arg in &func.args {
        types.insert(arg.ssa_var, arg.typ.clone());
    }

    for block in &func.blocks {
        for inst in &block.instructions {
            infer_inst_types(inst, &mut types);
        }
    }
    types
}

fn infer_inst_types(inst: &MirInstruction, types: &mut HashMap<usize, OnuType>) {
    use MirInstruction::*;
    match inst {
        BinaryOperation {
            dest, dest_type, ..
        } => {
            types.insert(*dest, dest_type.clone());
        }
        Call {
            dest,
            return_type,
            ..
        } => {
            types.insert(*dest, return_type.clone());
        }
        Assign { dest, src } => {
            let t = match src {
                MirOperand::Variable(ssa, _) => {
                    types.get(ssa).cloned().unwrap_or(OnuType::I64)
                }
                MirOperand::Constant(lit) => lit_type(lit),
            };
            types.insert(*dest, t);
        }
        Alloc { dest, .. } | StackAlloc { dest, .. } | GlobalAlloc { dest, .. } => {
            types.insert(*dest, OnuType::Ptr);
        }
        SaveArena { dest } => {
            types.insert(*dest, OnuType::Ptr);
        }
        PointerOffset { dest, .. } => {
            types.insert(*dest, OnuType::Ptr);
        }
        Load { dest, typ, .. } => {
            types.insert(*dest, typ.clone());
        }
        Promote { dest, to_type, .. } | BitCast { dest, to_type, .. } => {
            types.insert(*dest, to_type.clone());
        }
        FunnelShiftRight { dest, width, .. } => {
            types.insert(*dest, if *width <= 32 { OnuType::U32 } else { OnuType::U64 });
        }
        ConstantTableLoad { dest, .. } => {
            types.insert(*dest, OnuType::I64);
        }
        Tuple { dest, .. } => {
            types.insert(*dest, OnuType::Strings); // i32 ptr to struct
        }
        Index { dest, subject, index } => {
            // Field 0 = len (i64), field 1 = ptr (i32), field 2 = is_dynamic (i32)
            let t = match index {
                0 => OnuType::I64,
                _ => OnuType::Ptr,
            };
            types.insert(*dest, t);
        }
        _ => {}
    }
}

fn lit_type(lit: &MirLiteral) -> OnuType {
    match lit {
        MirLiteral::I64(_) => OnuType::I64,
        MirLiteral::F64(_) => OnuType::F64,
        MirLiteral::Boolean(_) => OnuType::Boolean,
        MirLiteral::Text(_) => OnuType::Strings,
        MirLiteral::Nothing => OnuType::Nothing,
        MirLiteral::WideInt(_, bits) => OnuType::WideInt(*bits),
    }
}

// ── Function signature helper ─────────────────────────────────────────────────

fn fn_sig(func: &MirFunction) -> (Vec<ValType>, Vec<ValType>) {
    let params: Vec<ValType> = func
        .args
        .iter()
        .map(|a| mir_to_wasm_type(&a.typ))
        .collect();
    let results: Vec<ValType> = match &func.return_type {
        OnuType::Nothing => vec![],
        t => vec![mir_to_wasm_type(t)],
    };
    (params, results)
}
