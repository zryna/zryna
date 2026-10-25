//! Branch alternatives replay independently; surviving paths join owner availability.
use super::{
    Checker, Failure, Statement, Use, charge, check_depth, clone_type, expressions, reject,
    resolve, symbolic_fact,
};
impl Checker<'_, '_, '_> {
    pub(super) fn block(&mut self, at: u32, depth: usize) -> Result<bool, Failure> {
        check_depth(depth)?;
        charge(self.spent, 1)?;
        let block = self.original.body.blocks.get(at as usize).ok_or(Failure::InternalFailure)?;
        let locals = self.locals.len();
        let mut returned = false;
        for at in &block.statements {
            charge(self.spent, 1)?;
            if returned {
                return Err(reject("original unreachable statements are not yet checked"));
            }
            let statement =
                self.original.body.statements.get(*at as usize).ok_or(Failure::InternalFailure)?;
            returned = match &statement.kind {
                Statement::LocalDeclaration { mutable, name, type_syntax, initializer, .. } => {
                    let expected = resolve(&self.resolver, *type_syntax, self.spent)?;
                    if matches!(expected, super::Closed::Borrow(..)) {
                        self.borrow_binding(&name.text, expected, *mutable, *initializer, depth)?;
                        continue;
                    }
                    let actual = self.expression(*initializer, Use::Move, depth + 1)?;
                    self.expect(&actual, &expected, statement.span)?;
                    symbolic_fact(self.resolver.originals, self.owner, &expected, self.spent)?;
                    self.bind_mutable(&name.text, expected, *mutable)?;
                    false
                }
                Statement::Return { value, .. } => {
                    let usage = if matches!(self.result, super::Closed::Borrow(..)) {
                        Use::Loan
                    } else {
                        Use::Move
                    };
                    let actual = self.expression(*value, usage, depth + 1)?;
                    if matches!(actual, super::Closed::Borrow(..)) {
                        return Err(self.error(
                            self.original.body.expressions[*value as usize].span,
                            "ZRYNA-M7007",
                            "an original loan cannot escape its function",
                        ));
                    }
                    let expected = clone_type(&self.result, self.spent)?;
                    self.expect(&actual, &expected, statement.span)?;
                    true
                }
                Statement::Block { block } => self.block(*block, depth + 1)?,
                Statement::If { condition, then_block, else_clause, .. } => {
                    let condition = self.expression(*condition, Use::Move, depth + 1)?;
                    let expected = expressions::scalar(0, self.spent)?;
                    self.expect(&condition, &expected, statement.span)?;
                    let before = self.snapshot()?;
                    let yes_returns = self.block(*then_block, depth + 1)?;
                    let yes = self.snapshot()?;
                    self.restore(&before)?;
                    let no_returns = if let Some(clause) = else_clause {
                        self.block(clause.block, depth + 1)?
                    } else {
                        false
                    };
                    let no = self.snapshot()?;
                    self.restore(&before)?;
                    if !yes_returns && !no_returns {
                        self.same_original(&yes, &no, statement.span)?;
                    }
                    self.restore(if yes_returns { &no } else { &yes })?;
                    yes_returns && no_returns
                }
                Statement::ExpressionStatement { expression, .. } => {
                    self.expression(*expression, Use::Move, depth + 1)?;
                    false
                }
                Statement::WeakUpgrade { .. } => self.upgrade(statement, depth)?,
                Statement::Assignment { target, value, .. } => {
                    self.assign(*target, *value, depth)?;
                    false
                }
                Statement::While { condition, body_block, keyword_span, .. } => {
                    self.loop_statement(*condition, *body_block, *keyword_span, depth)?;
                    false
                }
            };
        }
        self.locals.truncate(locals);
        Ok(returned)
    }
}
