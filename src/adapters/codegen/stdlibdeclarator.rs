use inkwell::context::Context;
use inkwell::module::{Module, Linkage};

pub struct StdlibDeclarator;

impl StdlibDeclarator {
    pub fn declare_all<'ctx>(context: &'ctx Context, module: &Module<'ctx>) {
        let i8_ptr = context.i8_type().ptr_type(inkwell::AddressSpace::default());
        let i64_type = context.i64_type();

        let malloc_type = i8_ptr.fn_type(&[i64_type.into()], false);
        module.add_function("malloc", malloc_type, Some(Linkage::External));

        let free_type = context.void_type().fn_type(&[i8_ptr.into()], false);
        module.add_function("free", free_type, Some(Linkage::External));

        let printf_type = context.i32_type().fn_type(&[i8_ptr.into()], true);
        module.add_function("printf", printf_type, Some(Linkage::External));

        let puts_type = context.i32_type().fn_type(&[i8_ptr.into()], false);
        module.add_function("puts", puts_type, Some(Linkage::External));

        let sprintf_type = context.i32_type().fn_type(&[i8_ptr.into(), i8_ptr.into()], true);
        module.add_function("sprintf", sprintf_type, Some(Linkage::External));

        let strlen_type = context.i64_type().fn_type(&[i8_ptr.into()], false);
        module.add_function("strlen", strlen_type, Some(Linkage::External));
    }
}
