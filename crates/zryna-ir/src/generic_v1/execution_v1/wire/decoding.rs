//! Complete-envelope decoder; no partially decoded carrier escapes any failure.
use super::{Failure, HEADER, VERSION, invalid, raw, reader::Reader, reserve};
use crate::data_ownership_v1 as m3;
impl Reader<'_> {
    pub(super) fn span(&mut self) -> Result<zryna_source::UntrustedSpan, Failure> {
        Ok(zryna_source::UntrustedSpan { file: self.u32()?, start: self.u32()?, end: self.u32()? })
    }
    pub(super) fn i32(&mut self) -> Result<i32, Failure> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into().map_err(|_| invalid())?))
    }
    pub(super) fn ids(&mut self) -> Result<Vec<u32>, Failure> {
        self.plain(m3::MAX_BLOCK_PARAMETERS, 4, Self::u32)
    }
    pub(super) fn literal(&mut self) -> Result<Vec<u8>, Failure> {
        let bytes = self.blob_slice(64 * 1024)?;
        self.totals.literal(bytes.len())?;
        std::str::from_utf8(bytes).map_err(|_| invalid())?;
        let mut value = reserve(bytes.len())?;
        value.extend_from_slice(bytes);
        Ok(value)
    }
    fn key(&mut self, function: bool) -> Result<Vec<u8>, Failure> {
        use crate::generic_v1::keys::{self, Domain};
        let bytes = self.blob_slice(4096)?;
        let domain = if function {
            match bytes.first() {
                Some(0x40) => Domain::FunctionInstance,
                Some(0x41) => Domain::SourceRoot,
                _ => return Err(invalid()),
            }
        } else {
            Domain::Type
        };
        keys::decode(bytes, domain)?;
        if function && bytes.first() == Some(&0x40) {
            self.totals.generic_function()?;
        }
        let mut key = reserve(bytes.len())?;
        key.extend_from_slice(bytes);
        Ok(key)
    }
    fn export(&mut self) -> Result<String, Failure> {
        let bytes = self.blob_slice(128)?;
        if !bytes.is_ascii() {
            return Err(invalid());
        }
        let mut value = reserve(bytes.len())?;
        value.extend_from_slice(bytes);
        String::from_utf8(value).map_err(|_| invalid())
    }
    fn instruction(&mut self) -> Result<raw::Instruction, Failure> {
        let site_span = self.span()?;
        let result = self.option(|reader| {
            reader.totals.values(1)?;
            reader.value_definition()
        })?;
        Ok(raw::Instruction { site_span, result, operation: self.operation()? })
    }
    fn block(&mut self) -> Result<raw::Block, Failure> {
        Ok(raw::Block {
            id: self.u32()?,
            value_parameters: self.vector(
                m3::MAX_BLOCK_PARAMETERS,
                5,
                super::limits::Totals::values,
                Self::value_definition,
            )?,
            instructions: self.vector(
                m3::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION,
                14,
                super::limits::Totals::transitions,
                Self::instruction,
            )?,
            terminator: self.terminator()?,
        })
    }
    fn function(&mut self) -> Result<raw::Function, Failure> {
        self.totals.function();
        Ok(raw::Function {
            key: self.key(true)?,
            original_span: self.span()?,
            executable_export: self.option(Self::export)?,
            parameters: self.vector(
                m3::MAX_PARAMETERS_PER_FUNCTION,
                6,
                super::limits::Totals::parameters,
                Self::parameter,
            )?,
            result_type: self.ty()?,
            places: self.plain(m3::MAX_PLACES_PER_FUNCTION, 25, Self::place)?,
            loans: self.plain(super::MAX_CHILDREN, 27, Self::loan)?,
            blocks: self.vector(
                m3::MAX_BLOCKS_PER_FUNCTION,
                18,
                super::limits::Totals::blocks,
                Self::block,
            )?,
            cleanup_plans: self.plain(
                m3::MAX_CLEANUP_PLANS_PER_FUNCTION,
                20,
                Self::cleanup_plan,
            )?,
        })
    }
}
impl Reader<'_> {
    pub(super) fn program(&mut self) -> Result<raw::Program, Failure> {
        if self.take(HEADER.len())? != HEADER || self.u32()? != VERSION {
            return Err(invalid());
        }
        let contract = self.blob_slice(raw::CONTRACT.len())?;
        if contract != raw::CONTRACT.as_bytes() {
            return Err(invalid());
        }
        let mut language_contract = reserve(contract.len())?;
        language_contract.extend_from_slice(contract);
        let program = raw::Program {
            language_contract: String::from_utf8(language_contract).map_err(|_| invalid())?,
            modules: self.plain(m3::MAX_MODULES, 8, |item| {
                Ok(raw::Module { id: item.u32()?, functions: item.u32()? })
            })?,
            declarations: self.plain(m3::MAX_FUNCTIONS_PER_PROGRAM, 24, |item| {
                Ok(raw::Declaration {
                    module: item.u32()?,
                    function: item.u32()?,
                    parameters: item.u32()?,
                    span: item.span()?,
                })
            })?,
            type_keys: self.plain(m3::MAX_INSTANTIATED_TYPES, 5, |item| item.key(false))?,
            universe: self.array()?,
            linear32: self.array()?,
            linux_x86_64: self.array()?,
            entry_module: self.u32()?,
            functions: self.plain(m3::MAX_FUNCTIONS_PER_PROGRAM, 47, Reader::function)?,
        };
        if self.cursor != self.bytes.len() {
            return Err(invalid());
        }
        Ok(program)
    }
}
pub(super) fn decode(bytes: &[u8]) -> Result<raw::Program, Failure> {
    Reader::new(bytes).program()
}
