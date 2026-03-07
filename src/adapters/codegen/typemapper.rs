use crate::application::use_cases::registry_service::RegistryService;
use crate::domain::entities::types::OnuType;
use inkwell::context::Context;
use inkwell::types::{BasicType, BasicTypeEnum};

pub struct LlvmTypeMapper;

impl LlvmTypeMapper {
    pub fn onu_to_llvm<'ctx>(
        context: &'ctx Context,
        typ: &OnuType,
        registry: &RegistryService,
    ) -> Option<BasicTypeEnum<'ctx>> {
        match typ {
            OnuType::I8 | OnuType::U8 => Some(context.i8_type().as_basic_type_enum()),
            OnuType::I16 | OnuType::U16 => Some(context.i16_type().as_basic_type_enum()),
            OnuType::I32 | OnuType::U32 => Some(context.i32_type().as_basic_type_enum()),
            OnuType::I64 | OnuType::U64 => Some(context.i64_type().as_basic_type_enum()),
            OnuType::I128 | OnuType::U128 => Some(context.i128_type().as_basic_type_enum()),
            OnuType::WideInt(bits) => Some(context.custom_width_int_type(*bits).as_basic_type_enum()),
            OnuType::Boolean => Some(context.bool_type().as_basic_type_enum()),
            OnuType::Strings => {
                // Canonical 3-field struct: { i64 len, i8* ptr, i1 is_dynamic }
                let i64t = context.i64_type();
                let i8ptr = context.i8_type().ptr_type(inkwell::AddressSpace::default());
                let bool_t = context.bool_type();
                Some(
                    context
                        .struct_type(&[i64t.into(), i8ptr.into(), bool_t.into()], false)
                        .as_basic_type_enum(),
                )
            }
            OnuType::Tuple(elements) => {
                let llvm_elements: Vec<inkwell::types::BasicTypeEnum> = elements
                    .iter()
                    .map(|t| {
                        Self::onu_to_llvm(context, t, registry)
                            .unwrap_or(context.i64_type().as_basic_type_enum())
                    })
                    .collect();
                Some(
                    context
                        .struct_type(&llvm_elements, false)
                        .as_basic_type_enum(),
                )
            }
            OnuType::Shape(name) => {
                if let Some(shape_def) = registry.get_shape(name) {
                    let llvm_elements: Vec<inkwell::types::BasicTypeEnum> = shape_def
                        .fields
                        .iter()
                        .map(|(_, t)| {
                            Self::onu_to_llvm(context, t, registry)
                                .unwrap_or(context.i64_type().as_basic_type_enum())
                        })
                        .collect();
                    Some(
                        context
                            .struct_type(&llvm_elements, false)
                            .as_basic_type_enum(),
                    )
                } else {
                    Some(context.i64_type().as_basic_type_enum())
                }
            }
            // Raw byte-pointer (internal compiler type, used only in MemoPass-generated code).
            // A Ptr is an i8* in LLVM — the natural type for a bump-allocator arena cache buffer.
            OnuType::Ptr => Some(
                context
                    .i8_type()
                    .ptr_type(inkwell::AddressSpace::default())
                    .as_basic_type_enum(),
            ),
            OnuType::Nothing => None,

            _ => Some(context.i64_type().as_basic_type_enum()),
        }
    }
}
