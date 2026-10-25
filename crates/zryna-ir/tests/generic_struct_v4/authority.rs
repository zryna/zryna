//! Public issuer attacks using fresh branded source/layout/runtime authorities.
use super::fixture::Fixture;
use zryna_ir::generic_v1::{Failure, owned_v2 as owned, raw as core};
use zryna_layout::generic_v1::VerifiedLayouts;
use zryna_ownership_runtime_abi::generic_v1::VerifiedOwnershipRuntimeAbi;

pub(super) struct Authorities<'a> {
    pub(super) fixture: &'a Fixture,
    pub(super) linear: &'a VerifiedLayouts,
    pub(super) linux: &'a VerifiedLayouts,
    pub(super) runtime: &'a VerifiedOwnershipRuntimeAbi<'a>,
}
impl Authorities<'_> {
    pub(super) fn seal(&self, bytes: &[u8]) -> Result<(), Failure> {
        owned::verify_v4(
            owned::wire::v4::decode(bytes)?,
            &self.fixture.syntax,
            &self.fixture.sources,
            self.fixture.sources.verify_file_id(0).expect("entry"),
            self.linear,
            self.linux,
            self.runtime,
        )
        .map(|_| ())
    }
    pub(super) fn old_issuers_reject_copied_claims(&self, bytes: &[u8]) {
        let copied_claims =
            owned::wire::v4::decode(bytes).expect("fresh v4 wrapper").claims().clone();
        assert!(
            copied_claims.extensions.iter().all(Vec::is_empty),
            "pure Copy pass graph has no new operation"
        );
        for b in [
            owned::wire::encode(&copied_claims).expect("old raw opcode-free encoding"),
            owned::wire::v3::encode(&copied_claims).expect("old v3 raw claims"),
        ] {
            let decoded = if b.starts_with(owned::wire::HEADER) {
                owned::wire::decode(&b)
            } else {
                owned::wire::v3::decode(&b)
            }
            .expect("old raw decoder is not admission");
            assert!(
                owned::verify(
                    decoded,
                    &self.fixture.syntax,
                    &self.fixture.sources,
                    self.fixture.sources.verify_file_id(0).expect("entry"),
                    self.linear,
                    self.linux,
                    self.runtime
                )
                .is_err(),
                "old issuer still rejects Struct key0x10/0x12"
            );
        }
    }
    pub(super) fn construction_attacks(&self, c: &owned::raw::Program) {
        let operations = c.extensions.iter().flatten().map(|e| &e.operation).collect::<Vec<_>>();
        assert_eq!(
            operations,
            vec![
                &owned::raw::Operation::StructConstruct { ty: 3, fields: vec![0] },
                &owned::raw::Operation::CopyStructField { value: 1, ordinal: 0 }
            ]
        );
        for attack in 0..5 {
            let mut hostile = c.clone();
            match attack {
                0 => {
                    hostile.extensions[0][0].operation =
                        owned::raw::Operation::StructConstruct { ty: 3, fields: vec![] }
                }
                1 => {
                    hostile.extensions[0][0].operation =
                        owned::raw::Operation::StructConstruct { ty: 3, fields: vec![0, 0] }
                }
                2 => {
                    hostile.extensions[0][0].operation =
                        owned::raw::Operation::StructConstruct { ty: 1, fields: vec![0] }
                }
                3 => {
                    hostile.extensions[0][1].operation =
                        owned::raw::Operation::CopyStructField { value: 1, ordinal: 1 }
                }
                _ => {
                    hostile.graph.functions[0].blocks[0].instructions[1].operation =
                        core::Operation::I32Literal(7);
                }
            }
            let bytes = owned::wire::v4::encode(&hostile).expect("physically valid hostile claims");
            assert!(self.seal(&bytes).is_err(), "source/typed attack {attack}");
        }
    }
    pub(super) fn foreign_source_rejects(&self, bytes: &[u8]) {
        let foreign = self.fixture.fresh();
        assert!(
            owned::verify_v4(
                owned::wire::v4::decode(bytes).expect("raw replay"),
                &foreign.syntax,
                &foreign.sources,
                foreign.sources.verify_file_id(0).expect("foreign entry"),
                self.linear,
                self.linux,
                self.runtime
            )
            .is_err(),
            "equal text has foreign authority"
        );
    }
}
