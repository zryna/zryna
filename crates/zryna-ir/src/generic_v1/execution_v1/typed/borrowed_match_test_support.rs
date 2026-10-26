//! Private owning phase inputs only; no `SourceProof` or `VerifiedExecution` is constructed.
use crate::generic_v1::execution_v1::{
    Failure, frontier, places, raw,
    type_world::{Builder, Kind, TypeWorld},
    typed,
};
use zryna_source::UntrustedSpan;
pub(in crate::generic_v1::execution_v1) fn span(start: u32, end: u32) -> UntrustedSpan {
    UntrustedSpan { file: 0, start, end }
}
pub(in crate::generic_v1::execution_v1) fn value(id: u32, ty: u32) -> raw::ValueDefinition {
    raw::ValueDefinition { id, ty: raw::ValueType::Stored(ty) }
}
pub(in crate::generic_v1::execution_v1) fn instruction(
    operation: raw::Operation,
    result: Option<raw::ValueDefinition>,
) -> raw::Instruction {
    raw::Instruction { site_span: span(25, 26), result, operation }
}
pub(in crate::generic_v1::execution_v1) fn input(
    access: raw::Access,
    family: u8,
) -> (raw::Program, TypeWorld) {
    build(access, family, false)
}
fn closed_world(family: u8, affine: bool) -> (u32, TypeWorld) {
    let mut b = Builder::new();
    let integer = b
        .declare(if affine { Kind::String } else { Kind::I32 }, &[], &mut 0)
        .expect("actual closed referent");
    let kind = match family {
        0 => Kind::Option,
        1 => Kind::Result,
        n => Kind::Enum { module: 0, declaration: u32::from(n - 2), arity: 0 },
    };
    let arguments = if family == 0 {
        vec![integer]
    } else if family == 1 {
        vec![integer, integer]
    } else {
        vec![]
    };
    let enumeration = b.declare(kind, &arguments, &mut 0).expect("actual original closed head");
    if family >= 2 {
        b.set_members(enumeration, &[None, Some(integer)], &mut 0).expect("nominal variants");
    }
    b.declare(Kind::Bool, &[], &mut 0).expect("independent ordinary branch condition type");
    let world = b.seal(&mut 0).expect("actual checked source-world builder");
    (enumeration, world)
}
fn build(access: raw::Access, family: u8, affine: bool) -> (raw::Program, TypeWorld) {
    let (enumeration, world) = closed_world(family, affine);
    let members = world.members(enumeration).expect("exact declaration members");
    let mut loans = vec![raw::Loan {
        id: 0,
        referent: enumeration,
        access,
        region: raw::Region { span: span(0, 100), role: raw::RegionRole::Lexical },
        origin: raw::LoanOrigin::Formal(0),
    }];
    let mut arms = Vec::new();
    let mut blocks = Vec::new();
    let mut next_value = 0;
    for (ordinal, payload) in members.iter().enumerate() {
        let ordinal = u32::try_from(ordinal).expect("two exact variants");
        let mut instructions = Vec::new();
        let binding = payload.map(|ty| {
            let id = u32::try_from(loans.len()).expect("two selected payloads");
            loans.push(raw::Loan {
                id,
                referent: ty,
                access,
                region: raw::Region { span: span(20, 40), role: raw::RegionRole::SelectedMatchArm },
                origin: raw::LoanOrigin::StaticProjection {
                    parent: 0,
                    projection: raw::StaticProjection::EnumPayload(ordinal),
                },
            });
            instructions.push(instruction(
                raw::Operation::ReadLoan { loan: id },
                Some(value(next_value, ty)),
            ));
            next_value += 1;
            instructions.push(instruction(raw::Operation::EndLoan { loan: id }, None));
            raw::MatchBinding::Loan(raw::LoanDefinition { id, referent: ty, access })
        });
        instructions.push(instruction(raw::Operation::EndLoan { loan: 0 }, None));
        arms.push(raw::EnumArm {
            ordinal,
            binding,
            edge: raw::Edge { target: ordinal + 1, arguments: vec![] },
        });
        blocks.push(raw::Block {
            id: ordinal + 1,
            value_parameters: vec![],
            instructions,
            terminator: raw::Terminator::Return { value: None, cleanup: 0 },
        });
    }
    blocks.insert(
        0,
        raw::Block {
            id: 0,
            value_parameters: vec![],
            instructions: vec![],
            terminator: raw::Terminator::ClosedEnumMatch {
                ty: enumeration,
                scrutinee: raw::PlaceOrLoan::Loan(0),
                mode: if access == raw::Access::Shared {
                    raw::MatchMode::SharedLoan
                } else {
                    raw::MatchMode::ExclusiveLoan
                },
                arms,
            },
        },
    );
    let function = raw::Function {
        key: vec![],
        original_span: span(0, 100),
        executable_export: None,
        parameters: vec![raw::Parameter::Loan(raw::LoanDefinition {
            id: 0,
            referent: enumeration,
            access,
        })],
        result_type: raw::ValueType::Unit,
        places: vec![],
        loans,
        blocks,
        cleanup_plans: vec![raw::CleanupPlan { id: 0, span: span(0, 100), actions: vec![] }],
    };
    (
        raw::Program {
            language_contract: raw::CONTRACT.into(),
            modules: vec![],
            declarations: vec![],
            type_keys: world.keys().map(<[u8]>::to_vec).collect(),
            universe: [0; 32],
            linear32: [0; 32],
            linux_x86_64: [0; 32],
            entry_module: 0,
            functions: vec![function],
        },
        world,
    )
}
pub(in crate::generic_v1::execution_v1) fn checked(
    f: &raw::Function,
    world: &TypeWorld,
) -> Result<typed::Function, Failure> {
    let graph = typed::cfg::Graph::build(f, world.len(), &mut 0)?;
    let values = graph.types().collect::<Vec<_>>();
    let places = places::check(f, &values, world, &mut 0)?;
    Ok(typed::Function {
        graph,
        values,
        places,
        callees: vec![],
        sites: vec![],
        may_fail: false,
        upgrades: vec![],
        match_scopes: Vec::new(),
    })
}
pub(in crate::generic_v1::execution_v1) fn inputs(
    p: &raw::Program,
    world: &TypeWorld,
) -> Result<typed::Typed, Failure> {
    let mut f = checked(&p.functions[0], world)?;
    let mut sites = Vec::new();
    for (block, b) in p.functions[0].blocks.iter().enumerate() {
        let mut row = Vec::new();
        for instruction in 0..b.instructions.len() {
            row.push(sites.len());
            sites.push(typed::Site { function: 0, block, instruction });
        }
        f.sites.push(row);
        f.callees.push(vec![None; b.instructions.len()]);
        f.upgrades.push(None);
    }
    Ok(typed::Typed {
        functions: vec![f],
        sites,
        frontier: frontier::Table::build(world, &mut 0)?,
        facts: (0..world.len())
            .map(|id| world.fact(u32::try_from(id).expect("bounded type")).expect("checked fact"))
            .collect(),
    })
}
pub(in crate::generic_v1::execution_v1) fn arms(f: &mut raw::Function) -> &mut Vec<raw::EnumArm> {
    let raw::Terminator::ClosedEnumMatch { arms, .. } = &mut f.blocks[0].terminator else {
        panic!("exact fixture match")
    };
    arms
}

pub(in crate::generic_v1::execution_v1) fn rooted() -> (raw::Program, TypeWorld) {
    let (mut p, w) = input(raw::Access::Exclusive, 0);
    let f = &mut p.functions[0];
    let enumeration = f.loans[0].referent;
    let payload = f.loans[1].referent;
    f.parameters = vec![raw::Parameter::Value(value(0, enumeration))];
    f.loans[0].origin = raw::LoanOrigin::Place(0);
    f.places = vec![
        raw::Place {
            id: 0,
            ty: enumeration,
            span: span(0, 100),
            kind: raw::PlaceKind::Parameter(0),
        },
        raw::Place {
            id: 1,
            ty: payload,
            span: span(20, 40),
            kind: raw::PlaceKind::EnumPayload { base: 0, variant: 1 },
        },
    ];
    f.blocks[0]
        .instructions
        .push(instruction(raw::Operation::BeginLoan { loan: 0, failure: None }, None));
    f.blocks[2].instructions[0].result = Some(value(1, payload));
    (p, w)
}
pub(in crate::generic_v1::execution_v1) fn cloning() -> (raw::Program, TypeWorld) {
    let (mut p, w) = build(raw::Access::Shared, 0, true);
    let f = &mut p.functions[0];
    let payload = f.loans[1].referent;
    f.places.push(raw::Place {
        id: 0,
        ty: payload,
        span: span(25, 26),
        kind: raw::PlaceKind::Temporary(0),
    });
    f.blocks[2].instructions[0].operation = raw::Operation::Clone {
        source: raw::PlaceOrLoan::Loan(1),
        outer_failure: Some(1),
        prefix_failure: Some(2),
    };
    f.blocks[2].instructions.insert(1, instruction(raw::Operation::DropPlace { place: 0 }, None));
    f.cleanup_plans.push(raw::CleanupPlan {
        id: 1,
        span: span(25, 26),
        actions: vec![raw::CleanupAction::EndLoan(1), raw::CleanupAction::EndLoan(0)],
    });
    f.cleanup_plans.push(raw::CleanupPlan {
        id: 2,
        span: span(25, 26),
        actions: vec![
            raw::CleanupAction::DropInitializedPrefix {
                destination: 0,
                preparation_site: raw::PreparationSite { function: 0, block: 2, instruction: 0 },
            },
            raw::CleanupAction::EndLoan(1),
            raw::CleanupAction::EndLoan(0),
        ],
    });
    (p, w)
}
