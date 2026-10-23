//! Rejection-only direct original-member evidence, never a source or layout authority.
use super::raw::NominalKind;

/// Raw original identity; equality includes the module, declaration and nominal kind.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NominalIdentity {
    /// Original module index.
    pub module: u32,
    /// Original data declaration index within that module.
    pub declaration: u32,
    /// Original Struct or enum identity.
    pub kind: NominalKind,
}

/// Raw identity of an opaque parameter in its original owning declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParameterIdentity {
    /// Exact original declaration owning this parameter.
    pub owner: NominalIdentity,
    /// Original ordered parameter slot.
    pub slot: u32,
}

/// A direct argument head, without descending constructor arguments or stored bodies.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Argument {
    /// One original opaque parameter, without a surrounding constructor.
    Parameter(ParameterIdentity),
    /// Any other argument shape, including a compound or constant argument.
    Other,
}

/// Untrusted mathematical evidence for one actual original stored member root.
///
/// Source adapters must independently authenticate identities and the member-root
/// origin. This raw witness grants no source, closed-instance or layout authority.
#[derive(Clone, Debug)]
pub struct DirectMember<'a> {
    /// Original declaration whose actual field or payload is being examined.
    pub owner: NominalIdentity,
    /// Exact original generic arity; this proof only handles one or two parameters.
    pub parameters: u32,
    /// Resolved nominal head of that complete stored member root.
    pub target: NominalIdentity,
    /// Direct ordered argument heads of the root application.
    pub arguments: &'a [Argument],
}

/// Proves only an immediate by-value self edge for every closed substitution.
///
/// False means this restricted proof has no conclusion, never that a source type
/// is admitted. No argument tree or nominal body is traversed. Arity and length
/// guards precede at most two argument checks; no storage is allocated.
#[must_use]
pub fn rejects_direct_self_member(witness: &DirectMember<'_>) -> bool {
    if !matches!((witness.parameters, witness.arguments.len()), (1, 1) | (2, 2))
        || witness.owner != witness.target
    {
        return false;
    }
    witness.arguments.iter().zip([0, 1]).all(|(argument, slot)| {
        matches!(argument, Argument::Parameter(parameter)
            if parameter.owner == witness.owner && parameter.slot == slot)
    })
}

#[cfg(test)]
#[path = "original_cycles/tests.rs"]
mod tests;
