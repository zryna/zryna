//! Completed descriptions reuse the proof's exact target and ordered member terms.
use super::syntax::{Resolved, push};
use super::{Budget, Closed, Engine, Facts, Failure, add, reject};
use zryna_syntax::v5::RawDataDeclarationKind;

pub(in super::super) struct Description<'b> {
    pub ty: Closed,
    pub members: Vec<(&'b str, Option<Closed>)>,
    pub structure: bool,
}
impl<'b> Facts<'_, 'b> {
    pub(in super::super) fn named(
        &self,
        module: u32,
        name: &str,
        keys: &[Vec<u8>],
        credit: &mut Budget,
    ) -> Result<Description<'b>, Failure> {
        let mut checked = *credit;
        let mut engine = Engine::new(self.originals);
        let (tag, module, index, count) = engine.head(module, name, &mut checked)?;
        if count != keys.len() {
            return Err(reject("constructor differs from exact original arity"));
        }
        let mut arguments = [0; 2];
        for (id, key) in arguments[..count].iter_mut().zip(keys) {
            *id = engine.decode(key, self.owner, self.symbolic, &mut checked)?;
        }
        let root = engine.application(tag, module, index, &arguments[..count], &mut checked)?;
        let description = engine.description(root, &mut checked)?;
        *credit = checked;
        Ok(description)
    }
    pub(in super::super) fn stored(
        &self,
        key: &[u8],
        credit: &mut Budget,
    ) -> Result<Description<'b>, Failure> {
        let mut checked = *credit;
        let mut engine = Engine::new(self.originals);
        let root = engine.decode(key, self.owner, self.symbolic, &mut checked)?;
        let description = engine.description(root, &mut checked)?;
        *credit = checked;
        Ok(description)
    }
}
impl<'b> Engine<'_, 'b> {
    fn description(&mut self, root: u32, credit: &mut Budget) -> Result<Description<'b>, Failure> {
        self.prove(root, credit)?;
        let key = self.terms[root as usize].key;
        if !matches!(key[0], 0x10..=0x15) {
            return Err(reject("description requires an original or compiler family"));
        }
        let mut members = Vec::new();
        if matches!(key[0], 0x14 | 0x15) {
            let names = if key[0] == 0x14 { ["none", "some"] } else { ["ok", "err"] };
            for (ordinal, name) in names.into_iter().enumerate() {
                let child = if key[0] == 0x14 && ordinal == 0 {
                    None
                } else {
                    Some(key[4 + if key[0] == 0x14 { 0 } else { ordinal }])
                };
                let ty = child.map(|id| self.closed(Resolved::Stored(id), credit)).transpose()?;
                push(&mut members, (name, ty), credit)?;
            }
        } else {
            credit.nominal(1)?;
            let original =
                &self.originals.units[key[1] as usize].data_declarations[key[2] as usize];
            let count = self.terms[root as usize].members.len();
            for cursor in 0..count {
                credit.nominal(1)?;
                let name = match &original.kind {
                    RawDataDeclarationKind::Struct { fields, .. } => {
                        fields[cursor].name.text.as_str()
                    }
                    RawDataDeclarationKind::Enum { variants, .. } => {
                        variants[cursor].name.text.as_str()
                    }
                };
                let child = self.terms[root as usize].members[cursor];
                let ty = child.map(|id| self.closed(Resolved::Stored(id), credit)).transpose()?;
                push(&mut members, (name, ty), credit)?;
            }
        }
        credit.nominal(add(1, members.len())?)?;
        Ok(Description {
            ty: Closed::Stored(self.encode(root, credit)?),
            members,
            structure: matches!(key[0], 0x10 | 0x12),
        })
    }
}
