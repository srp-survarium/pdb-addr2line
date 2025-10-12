#![expect(unused_imports)]

use crate::Result;
use crate::{TypeFormatter, TypeFormatterForModule};

use pdb::{
    ArgumentList, ArrayType, ClassKind, ClassType, CrossModuleExports, CrossModuleImports,
    CrossModuleRef, DebugInformation, FallibleIterator, FunctionAttributes, IdData, IdIndex,
    IdInformation, Item, ItemFinder, ItemIndex, ItemIter, MachineType, MemberFunctionType,
    ModifierType, Module, ModuleInfo, PointerMode, PointerType, PrimitiveKind, PrimitiveType,
    ProcedureType, RawString, StringTable, TypeData, TypeIndex, TypeInformation, UnionType,
    Variant,
};

#[derive(Debug, Clone)]
pub struct Function {
    pub return_type: ReturnType,
    pub name: String,
    pub arg_types: Vec<String>,
    pub attrs: AttributeFlags,
}

#[derive(Debug, Clone)]
pub enum ReturnType {
    Constructor,
    Destructor,
    Type(String),
}

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug)]
    pub struct AttributeFlags: u32 {
        const IS_STATIC   = 1 << 0;
        const IS_CONST    = 1 << 7;

        // Are not filled in!
        const IS_VIRTUAL  = 1 << 1;
        const IS_INLINE   = 1 << 2;
        const IS_OVERRIDE = 1 << 3;
        const IS_PURE     = 1 << 4;
        const IS_FINAL    = 1 << 5;
    }
}

//
//
//

impl<'a, 's> TypeFormatter<'a, 's> {
    pub fn parse_function(
        &self,
        name: &str,
        module_index: usize,
        function_type_index: pdb::TypeIndex,
    ) -> Result<Function> {
        self.for_module(module_index, |tf| {
            tf.parse_function(name, function_type_index)
        })
    }
}

impl<'a, 's> TypeFormatterForModule<'_, 'a, 's> {
    pub fn parse_function(
        &mut self,
        name: &str,
        function_type_index: pdb::TypeIndex,
    ) -> Result<Function> {
        if function_type_index == pdb::TypeIndex(0) {
            unreachable!("Deal with this when it happens")
        }

        let mut attrs = AttributeFlags::empty();

        let extra_first_arg: Option<pdb::TypeIndex>;
        let return_type: Option<pdb::TypeIndex>;
        let attributes: pdb::FunctionAttributes;
        let argument_list: pdb::TypeIndex;

        match self.parse_type_index(function_type_index)? {
            TypeData::MemberFunction(pdb::MemberFunctionType {
                return_type: return_type_t,
                attributes: attributes_t,
                argument_list: argument_list_t,
                class_type,
                this_pointer_type,
                this_adjustment: _,
                parameter_count: _,
            }) => {
                return_type = Some(return_type_t);
                attributes = attributes_t;
                argument_list = argument_list_t;

                match this_pointer_type {
                    None => {
                        attrs |= AttributeFlags::IS_STATIC;
                        extra_first_arg = None;
                    }
                    Some(this_type) => {
                        let (is_const, this_arg) =
                            self.get_class_constness_and_extra_arguments(this_type, class_type)?;

                        if is_const {
                            attrs |= AttributeFlags::IS_CONST;
                        }
                        extra_first_arg = this_arg
                    }
                }
            }
            TypeData::Procedure(pdb::ProcedureType {
                return_type: return_type_t,
                attributes: attributes_t,
                parameter_count: _,
                argument_list: argument_list_t,
            }) => {
                return_type = return_type_t;
                attributes = attributes_t;
                argument_list = argument_list_t;
                extra_first_arg = None;
            }
            _ => unreachable!("Deal with this when it happens"),
        }

        let return_type = self.parse_return_type(name, return_type, attributes)?;
        let name = self.parse_name(name);

        let argument_list = match self.parse_type_index(argument_list)? {
            TypeData::ArgumentList(t) => t,
            _ => unreachable!("Deal with this when it happens"),
        };

        let arg_types = self.parse_arg_list(extra_first_arg, argument_list)?;

        Ok(Function {
            return_type,
            name,
            arg_types,
            attrs,
        })
    }

    //
    //
    //

    pub fn parse_name(&mut self, name: &str) -> String {
        if name.is_empty() {
            "<name omitted>".to_string()
        } else {
            name.to_string()
        }
    }

    pub fn parse_return_type(
        &mut self,
        name: &str,
        type_index: Option<TypeIndex>,
        attrs: pdb::FunctionAttributes,
    ) -> Result<ReturnType> {
        let return_type = match () {
            () if attrs.is_constructor() => ReturnType::Constructor,
            () if name.starts_with("~") => ReturnType::Destructor,
            () if type_index.is_some() => ReturnType::Type(self.parse_type(type_index.unwrap())?),
            () => unreachable!(),
        };
        Ok(return_type)
    }

    pub fn parse_arg_list(
        &mut self,
        this_arg: Option<TypeIndex>,
        list: pdb::ArgumentList,
    ) -> Result<Vec<String>> {
        let mut args = Vec::with_capacity(this_arg.is_some() as usize + list.arguments.len());

        if let Some(index) = this_arg {
            args.push(self.parse_type(index)?);
        }

        for index in list.arguments {
            args.push(self.parse_type(index)?);
        }

        Ok(args)
    }

    pub fn parse_type(&mut self, index: TypeIndex) -> Result<String> {
        let mut type_ = String::new();
        self.emit_type_index(&mut type_, index)?;
        Ok(type_)
    }
}
