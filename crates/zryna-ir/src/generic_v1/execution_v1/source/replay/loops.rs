//! Header conditions repeat; body-only storage/loans end before each surviving latch.
use super::{Body, Failure, State, UntrustedSpan, charge, raw};
impl Body<'_, '_> {
    pub(super) fn source_loop(
        &mut self,
        condition: u32,
        body: u32,
        span: UntrustedSpan,
        depth: usize,
    ) -> Result<(), Failure> {
        let saved = self.snapshot()?;
        let locals = saved.locals.len();
        let condition_places = self.output.places.len();
        let preheader = self.block;
        let header = self.new_block()?;
        self.output.blocks[preheader].terminator = raw::Terminator::Jump(raw::Edge {
            target: u32::try_from(header).map_err(|_| Failure::InternalFailure)?,
            arguments: Vec::new(),
        });
        self.block = header;
        let value = self.expression(condition, depth + 1)?;
        if value.ty != self.primitive(0)? {
            return Err(self.error(
                self.original.body.expressions[condition as usize].span,
                "ZRYNA-M7006",
                "loop condition requires exact bool",
            ));
        }
        // Copy SSA stays a value even when its addressable temporary is released.
        // Dominance authenticates its use; cleanup cannot grant a body-only definition.
        self.close_scope(locals, condition_places, span)?;
        self.loop_header_state(&saved, span)?;
        let condition_exit = self.block;
        let first_body_place = self.output.places.len();
        let body_entry = self.new_block()?;
        self.block = body_entry;
        let returns = self.block_statements(body, depth + 1)?;
        if !returns {
            self.close_scope(locals, first_body_place, span)?;
            self.loop_header_state(&saved, span)?;
            self.output.blocks[self.block].terminator = raw::Terminator::Jump(raw::Edge {
                target: u32::try_from(header).map_err(|_| Failure::InternalFailure)?,
                arguments: Vec::new(),
            });
        }
        self.restore(saved)?;
        let exit = self.new_block()?;
        self.block = exit;
        self.output.blocks[condition_exit].terminator = raw::Terminator::Branch {
            condition: value.id,
            yes: raw::Edge {
                target: u32::try_from(body_entry).map_err(|_| Failure::InternalFailure)?,
                arguments: Vec::new(),
            },
            no: raw::Edge {
                target: u32::try_from(exit).map_err(|_| Failure::InternalFailure)?,
                arguments: Vec::new(),
            },
        };
        Ok(())
    }
    fn loop_header_state(&mut self, saved: &State<'_>, span: UntrustedSpan) -> Result<(), Failure> {
        self.same_state(saved)?;
        charge(self.spent, self.state.live.len() + self.state.loans.len() + saved.locals.len())?;
        let unchanged = self
            .state
            .live
            .iter()
            .enumerate()
            .all(|(id, live)| *live == saved.live.get(id).copied().unwrap_or(false))
            && self
                .state
                .loans
                .iter()
                .enumerate()
                .all(|(id, live)| *live == saved.loans.get(id).copied().unwrap_or(false))
            && self.state.locals.len() == saved.locals.len()
            && self.state.locals.iter().zip(&saved.locals).all(|(a, b)| {
                a.name == b.name && a.source == b.source && a.ty == b.ty && a.mutable == b.mutable
            });
        if !unchanged {
            return Err(self.error(
                span,
                "ZRYNA-M7007",
                "loop source state differs at repeated header",
            ));
        }
        Ok(())
    }
}
