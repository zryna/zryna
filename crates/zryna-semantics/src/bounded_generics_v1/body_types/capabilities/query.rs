//! Evaluate checked symbolic substitutions without unfolding nominal payloads.

use super::{
    BodyTypeFailure, DeclarationIdentity, Graph, Head, Kind, Origin, Scalar, Status, Tables, Ty,
    resources, substitution,
};

impl Graph {
    pub(in super::super) fn query(
        &self,
        tables: &Tables,
        function: DeclarationIdentity,
        ty: Ty,
        clone: bool,
        assume_parameters: bool,
    ) -> Result<Status, BodyTypeFailure> {
        // Body descriptors are a source/substitution DAG. Use explicit evaluation frames; no
        // nominal field unfolding or call-body recursion is performed by this query.
        enum Work {
            Type(Ty, bool),
            Finish(Head, bool),
        }
        let mut work = resources::reserve(129)?;
        let mut results = resources::reserve(129)?;
        work.push(Work::Type(ty, clone));
        while let Some(item) = work.pop() {
            match item {
                Work::Type(ty, clone) => {
                    let Some(ty) = substitution::normalize(tables, function, ty)? else {
                        results.try_reserve(1).map_err(|_| BodyTypeFailure::AllocationFailure)?;
                        results.push(Status::Unknown);
                        continue;
                    };
                    if let Origin::Source { module, occurrence } = ty.origin {
                        let row = self.source(module, occurrence);
                        let value = if ty.environment == 0 {
                            self.rows[row]
                                .values
                                .get(usize::from(assume_parameters) * 30 + usize::from(clone))
                        } else {
                            let environment = &tables.function(function).environments
                                [ty.environment as usize - 1];
                            if tables.sources[module as usize][occurrence as usize].owner
                                != environment.owner
                            {
                                return Err(BodyTypeFailure::InternalFailure);
                            }
                            self.refine(
                                row,
                                environment.capabilities[usize::from(assume_parameters)],
                                clone,
                            )
                        };
                        results.try_reserve(1).map_err(|_| BodyTypeFailure::AllocationFailure)?;
                        results.push(value);
                        continue;
                    }
                    let Some(head) = substitution::head(tables, function, ty)? else {
                        results.try_reserve(1).map_err(|_| BodyTypeFailure::AllocationFailure)?;
                        results.push(Status::Unknown);
                        continue;
                    };
                    let immediate = match head.kind {
                        Kind::Parameter(_) => {
                            Some(if assume_parameters { Status::True } else { Status::False })
                        }
                        Kind::Borrow | Kind::BorrowMut | Kind::Function(_) => Some(Status::False),
                        Kind::Scalar(Scalar::String) | Kind::Shared | Kind::Weak => {
                            Some(if clone { Status::True } else { Status::False })
                        }
                        Kind::Scalar(_) => Some(Status::True),
                        Kind::Vec if !clone => Some(Status::False),
                        _ => None,
                    };
                    if let Some(value) = immediate {
                        results.try_reserve(1).map_err(|_| BodyTypeFailure::AllocationFailure)?;
                        results.push(value);
                        continue;
                    }
                    work.try_reserve(5).map_err(|_| BodyTypeFailure::AllocationFailure)?;
                    work.push(Work::Finish(head, clone));
                    for child in head.children.into_iter().rev().flatten() {
                        if matches!(head.kind, Kind::Nominal(_)) {
                            work.push(Work::Type(child, true));
                            work.push(Work::Type(child, false));
                        } else {
                            work.push(Work::Type(child, clone));
                        }
                    }
                }
                Work::Finish(head, clone) => {
                    let mut result = Status::True;
                    if let Kind::Nominal(owner) = head.kind {
                        let mut bits = [Status::False; 4];
                        for index in (0..2).rev() {
                            if head.children[index].is_some() {
                                bits[index * 2 + 1] =
                                    results.pop().ok_or(BodyTypeFailure::InternalFailure)?;
                                bits[index * 2] =
                                    results.pop().ok_or(BodyTypeFailure::InternalFailure)?;
                            }
                        }
                        result = self.apply(owner, bits, clone);
                    } else {
                        for _ in head.children.into_iter().flatten() {
                            result =
                                result.and(results.pop().ok_or(BodyTypeFailure::InternalFailure)?);
                        }
                    }
                    results.try_reserve(1).map_err(|_| BodyTypeFailure::AllocationFailure)?;
                    results.push(result);
                }
            }
        }
        results.pop().ok_or(BodyTypeFailure::InternalFailure)
    }
}
