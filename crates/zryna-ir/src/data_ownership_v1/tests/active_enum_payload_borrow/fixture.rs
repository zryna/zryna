use super::super::indexed_borrow_fixture::{Container, Element, Fixture};
use super::super::*;
use zryna_layout::TypeCategory;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::data_ownership_v1::tests) enum Mode {
    Shared,
    Exclusive,
}

impl Mode {
    pub(in crate::data_ownership_v1::tests) fn access(self) -> raw::BorrowAccess {
        match self {
            Self::Shared => raw::BorrowAccess::Shared,
            Self::Exclusive => raw::BorrowAccess::Exclusive,
        }
    }
}

pub(in crate::data_ownership_v1::tests) struct Seed {
    fixture: Fixture,
    mode: Mode,
    pub(in crate::data_ownership_v1::tests) root: raw::TypeId,
    pub(in crate::data_ownership_v1::tests) string: raw::TypeId,
}

impl Seed {
    pub(in crate::data_ownership_v1::tests) fn new(mode: Mode) -> Self {
        let fixture = Fixture::new(Container::Vec, Element::String);
        let string = raw::TypeId(
            fixture
                .linear
                .types()
                .find(|ty| ty.category() == TypeCategory::String)
                .expect("String")
                .id()
                .index(),
        );
        let root = fixture.root;
        Self { fixture, mode, root, string }
    }

    pub(in crate::data_ownership_v1::tests) fn program(&self) -> raw::Program {
        let mut raw = self.fixture.seed(self.mode.access());
        let function = &mut raw.modules[0].functions[0];
        let span = function.span;
        function.parameters.truncate(3);
        function.parameters[2].ty = self.fixture.integer;
        function.result = self.fixture.integer;
        function.places = vec![
            raw::Place {
                id: raw::PlaceId(0),
                ty: self.root,
                span,
                kind: raw::PlaceKind::Parameter(0),
            },
            raw::Place {
                id: raw::PlaceId(1),
                ty: self.root,
                span,
                kind: raw::PlaceKind::Parameter(1),
            },
            raw::Place {
                id: raw::PlaceId(2),
                ty: self.fixture.integer,
                span,
                kind: raw::PlaceKind::Parameter(2),
            },
            raw::Place {
                id: raw::PlaceId(3),
                ty: self.fixture.wrapper,
                span,
                kind: raw::PlaceKind::Temporary(raw::ValueId(3)),
            },
            raw::Place {
                id: raw::PlaceId(4),
                ty: self.root,
                span,
                kind: raw::PlaceKind::EnumPayload { base: raw::PlaceId(3), variant: 1 },
            },
        ];
        function.blocks[0].instructions = vec![
            raw::Instruction {
                result: Some(raw::ValueDefinition {
                    id: raw::ValueId(3),
                    ty: self.fixture.wrapper,
                    span,
                }),
                span,
                kind: raw::InstructionKind::EnumConstruct {
                    variant: 1,
                    payload: Some(raw::ValueId(0)),
                    cleanup: None,
                },
            },
            begin_borrow(0, 4, self.mode.access(), span),
        ];
        if self.mode == Mode::Exclusive {
            function.blocks[0].instructions.push(raw::Instruction {
                result: None,
                span,
                kind: raw::InstructionKind::BorrowReplace {
                    borrow: raw::BorrowId(0),
                    value: raw::ValueId(1),
                },
            });
        }
        function.blocks[0].instructions.push(end_borrow(0, span));
        let mut actions = vec![raw::DropAction::DropPlace(raw::PlaceId(3))];
        if self.mode == Mode::Shared {
            actions.push(raw::DropAction::DropPlace(raw::PlaceId(1)));
        }
        function.cleanup_plans =
            vec![raw::CleanupPlan { id: raw::CleanupPlanId(0), span, actions }];
        function.blocks[0].terminators[0].kind =
            raw::Terminator::Return { value: raw::ValueId(2), cleanup: raw::CleanupPlanId(0) };
        raw
    }

    pub(in crate::data_ownership_v1::tests) fn check(
        &self,
        raw: raw::Program,
    ) -> Result<super::super::super::VerifiedProgram, Vec<zryna_diagnostics::Diagnostic>> {
        verify(
            raw,
            &self.fixture.sources,
            self.fixture.sources.verify_file_id(0).expect("entry"),
            self.fixture.linear.clone(),
            self.fixture.linux.clone(),
        )
    }
}
