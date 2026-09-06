use super::*;

#[derive(Clone, Copy, Debug)]
pub(in crate::data_ownership_v1) enum Payload {
    Bool,
    I32,
    String,
    Struct,
    Enum,
    EmptyArray,
    Array,
    Vec,
    Shared,
    Weak,
    NestedHandles,
}

impl Payload {
    pub(in crate::data_ownership_v1) const ALL: [Self; 11] = [
        Self::Bool,
        Self::I32,
        Self::String,
        Self::Struct,
        Self::Enum,
        Self::EmptyArray,
        Self::Array,
        Self::Vec,
        Self::Shared,
        Self::Weak,
        Self::NestedHandles,
    ];

    pub(super) fn setup(self, f: &mut Builder) -> (Ty, Vec<RawDataDeclaration>) {
        let ty = match self {
            Self::Bool => Ty::Named("bool"),
            Self::I32 => Ty::Named("i32"),
            Self::String => Ty::String,
            Self::Struct | Self::Enum => {
                let shape = if matches!(self, Self::Enum) {
                    shared_weak_fixture::NominalPayload::Enum
                } else {
                    shared_weak_fixture::NominalPayload::Struct
                };
                let declaration = shared_weak_fixture::nominal_payload(f, shape);
                return (Ty::Named("Payload"), vec![declaration]);
            }
            Self::EmptyArray => Ty::Array(Box::new(Ty::String), 0),
            Self::Array => Ty::Array(Box::new(Ty::String), 2),
            Self::Vec => Ty::Vec(Box::new(Ty::String)),
            Self::Shared => Ty::Shared(Box::new(Ty::String)),
            Self::Weak => Ty::Weak(Box::new(Ty::String)),
            Self::NestedHandles => {
                let handle = Ty::Shared(Box::new(Ty::Weak(Box::new(Ty::String))));
                Ty::Vec(Box::new(Ty::Array(Box::new(handle), 2)))
            }
        };
        (ty, vec![])
    }
}
