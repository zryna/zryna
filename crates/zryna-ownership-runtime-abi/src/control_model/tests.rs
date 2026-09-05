use super::*;
use crate::{raw_v1, verify_v1};
use zryna_layout::{TypeCategory, raw as layout};
use zryna_source::{SourceFileInput, SourceMap};

mod boundaries;
mod resources;

fn authorities() -> (VerifiedOwnershipRuntimeAbi, VerifiedLayouts, VerifiedLayouts) {
    let sources =
        SourceMap::build(vec![SourceFileInput { path: "main.zry".into(), text: "payload".into() }])
            .expect("source");
    let file = sources.verify_file_id(0).expect("file");
    let kinds = vec![
        layout::TypeKind::Bool,
        layout::TypeKind::I32,
        layout::TypeKind::String,
        layout::TypeKind::Shared { payload: layout::NodeId(2) },
        layout::TypeKind::Weak { payload: layout::NodeId(2) },
        layout::TypeKind::Struct {
            module: layout::ModuleId(0),
            declaration: 0,
            fields: vec![
                layout::Field { ordinal: 0, ty: layout::NodeId(3) },
                layout::Field { ordinal: 1, ty: layout::NodeId(4) },
            ],
        },
        layout::TypeKind::Enum {
            module: layout::ModuleId(0),
            declaration: 1,
            variants: vec![
                layout::Variant { ordinal: 0, payload: None },
                layout::Variant { ordinal: 1, payload: Some(layout::NodeId(2)) },
            ],
        },
        layout::TypeKind::FixedArray { element: layout::NodeId(2), length: 0 },
        layout::TypeKind::FixedArray { element: layout::NodeId(2), length: 2 },
        layout::TypeKind::Vec { element: layout::NodeId(2) },
    ];
    let mut types: Vec<_> = kinds
        .into_iter()
        .enumerate()
        .map(|(id, kind)| layout::TypeNode {
            id: layout::NodeId(u32::try_from(id).expect("small type")),
            span: matches!(kind, layout::TypeKind::Struct { .. } | layout::TypeKind::Enum { .. })
                .then(|| sources.span(file, 0, 7).expect("nominal span")),
            kind,
        })
        .collect();
    for payload in [0, 1, 3, 4, 5, 6, 7, 8, 9] {
        let id = layout::NodeId(u32::try_from(types.len()).expect("small type"));
        types.push(layout::TypeNode {
            id,
            span: None,
            kind: layout::TypeKind::Shared { payload: layout::NodeId(payload) },
        });
    }
    let graph = layout::Graph {
        modules: vec![layout::Module {
            id: layout::ModuleId(0),
            source_file: file,
            data_declarations: 2,
        }],
        program_roots: types.iter().map(|ty| ty.id).collect(),
        types,
    };
    let linear = zryna_layout::verify(&graph, &sources, StorageTarget::Linear32V1).expect("linear");
    let linux = zryna_layout::verify(&graph, &sources, StorageTarget::LinuxX8664V1).expect("linux");
    let abi = verify_v1(raw_v1(&linear, &linux), &linear, &linux).expect("ABI authority");
    (abi, linear, linux)
}

fn ty(layouts: &VerifiedLayouts, category: TypeCategory) -> TypeId {
    layouts.types().find(|ty| ty.category() == category).expect("type").id()
}
fn state(strong: u32, weak: u32, pending: bool, initialized: bool) -> ControlState {
    ControlState {
        strong_count: strong,
        weak_count: weak,
        pending_last_strong: pending,
        payload_initialized: initialized,
        allocated: weak != 0,
    }
}
fn construct(
    abi: &VerifiedOwnershipRuntimeAbi,
    layouts: &VerifiedLayouts,
    nodes: Vec<PayloadNode>,
    control: u32,
    owner: u32,
    base: u64,
) -> ControlEventKind {
    let payload = nodes.last().expect("root").ty;
    let layout = abi
        .control_layouts()
        .find(|layout| layout.payload() == payload && layout.target() == layouts.target())
        .expect("control layout");
    ControlEventKind::Construct {
        payload,
        nodes,
        status: RuntimeStatus::Ok,
        base,
        size: layout.size(),
        alignment: layout.alignment(),
        control: Some(control),
        owner: Some(owner),
    }
}
fn handle(
    operation: LogicalOperation,
    owner: u32,
    result: Option<u32>,
    boolean: Option<bool>,
    after: ControlState,
) -> ControlEventKind {
    ControlEventKind::Handle { operation, owner, result, boolean, after, status: RuntimeStatus::Ok }
}
fn trace(
    abi: &VerifiedOwnershipRuntimeAbi,
    layouts: &VerifiedLayouts,
    events: Vec<ControlEventKind>,
) -> ControlTrace {
    ControlTrace {
        invocation: 17,
        abi: abi.identity(),
        target: layouts.target(),
        events: events
            .into_iter()
            .enumerate()
            .map(|(site, kind)| ControlEvent { site: site as u64, kind })
            .collect(),
    }
}
fn seed(abi: &VerifiedOwnershipRuntimeAbi, layouts: &VerifiedLayouts) -> ControlEventKind {
    construct(
        abi,
        layouts,
        vec![PayloadNode { ty: ty(layouts, TypeCategory::String), kind: PayloadKind::String }],
        0,
        0,
        1024,
    )
}
fn reject(abi: &VerifiedOwnershipRuntimeAbi, layouts: &VerifiedLayouts, bad: &ControlTrace) {
    let first = verify_control_trace(abi, layouts, 17, &[], bad).expect_err("hostile model trace");
    assert_ne!(
        first.message(),
        "surviving external owners differ from the independent exit contract",
        "hostile operation must fail before final owner reconciliation"
    );
    assert_eq!(
        first,
        verify_control_trace(abi, layouts, 17, &[], bad).expect_err("deterministic rejection")
    );
    let valid = trace(abi, layouts, vec![seed(abi, layouts)]);
    assert_eq!(
        verify_control_trace(
            abi,
            layouts,
            17,
            &[ExpectedOwner { owner: 0, control: 0, weak: false }],
            &valid
        )
        .expect("recovery"),
        verify_control_trace(
            abi,
            layouts,
            17,
            &[ExpectedOwner { owner: 0, control: 0, weak: false }],
            &valid
        )
        .expect("same recovery")
    );
}

#[test]
fn control_model_clone_expiration_and_payload_before_implicit_weak_finish() {
    let (abi, linear, linux) = authorities();
    for layouts in [&linear, &linux] {
        let mut events = vec![
            seed(&abi, layouts),
            handle(LogicalOperation::StrongClone, 0, Some(1), None, state(2, 1, false, true)),
            handle(LogicalOperation::WeakDowngrade, 1, Some(2), None, state(2, 2, false, true)),
            handle(
                LogicalOperation::StrongReleaseBegin,
                1,
                None,
                Some(false),
                state(1, 2, false, true),
            ),
            handle(
                LogicalOperation::StrongReleaseBegin,
                0,
                None,
                Some(true),
                state(0, 2, true, true),
            ),
            ControlEventKind::DropPayloadNode { control: 0, node: 0 },
            ControlEventKind::Finish { control: 0, after: state(0, 1, false, false) },
            handle(LogicalOperation::WeakClone, 2, Some(3), None, state(0, 2, false, false)),
            ControlEventKind::Handle {
                operation: LogicalOperation::WeakUpgrade,
                owner: 2,
                result: None,
                boolean: None,
                status: RuntimeStatus::Expired,
                after: state(0, 2, false, false),
            },
            handle(LogicalOperation::WeakRelease, 3, None, Some(false), state(0, 1, false, false)),
            handle(LogicalOperation::WeakRelease, 2, None, Some(true), state(0, 0, false, false)),
        ];
        let valid = trace(&abi, layouts, events.clone());
        let proof =
            verify_control_trace(&abi, layouts, 17, &[], &valid).expect("exact release trace");
        assert_eq!(proof.states().collect::<Vec<_>>(), [state(0, 0, false, false)]);
        assert_eq!(proof.live_owners(), 0);
        events.swap(5, 6);
        reject(&abi, layouts, &trace(&abi, layouts, events));
        let mut forged = valid.clone();
        if let ControlEventKind::Handle { result, .. } = &mut forged.events[8].kind {
            *result = Some(4);
        }
        reject(&abi, layouts, &forged);
        let mut forged = valid.clone();
        forged.events.push(ControlEvent { site: 11, kind: valid.events[10].kind.clone() });
        reject(&abi, layouts, &forged);
    }
}

#[test]
fn control_model_provenance_topology_modes_and_allocation_failures_are_atomic() {
    let (abi, layouts, _) = authorities();
    let valid = trace(&abi, &layouts, vec![seed(&abi, &layouts)]);
    for mutation in 0..7 {
        let mut bad = valid.clone();
        let ControlEventKind::Construct { size, alignment, base, owner, nodes, .. } =
            &mut bad.events[0].kind
        else {
            unreachable!()
        };
        match mutation {
            0 => *size += 1,
            1 => *alignment *= 2,
            2 => *base = 0,
            3 => *owner = Some(1),
            4 => nodes[0].kind = PayloadKind::Handle(0),
            5 => bad.invocation += 1,
            6 => bad.events[0].site = 1,
            _ => unreachable!(),
        }
        reject(&abi, &layouts, &bad);
    }
    let mut overlap = seed(&abi, &layouts);
    if let ControlEventKind::Construct { control, owner, .. } = &mut overlap {
        *control = Some(1);
        *owner = Some(1);
    }
    reject(&abi, &layouts, &trace(&abi, &layouts, vec![seed(&abi, &layouts), overlap]));
    let mut failed = seed(&abi, &layouts);
    if let ControlEventKind::Construct { status, base, control, owner, .. } = &mut failed {
        *status = RuntimeStatus::Allocation;
        *base = 0;
        *control = None;
        *owner = None;
    }
    let recovery = trace(&abi, &layouts, vec![failed, seed(&abi, &layouts)]);
    assert_eq!(
        verify_control_trace(
            &abi,
            &layouts,
            17,
            &[ExpectedOwner { owner: 0, control: 0, weak: false }],
            &recovery
        )
        .expect("failed construction issues nothing")
        .live_owners(),
        1
    );
    let wrong_mode = trace(
        &abi,
        &layouts,
        vec![
            seed(&abi, &layouts),
            handle(LogicalOperation::WeakClone, 0, Some(1), None, state(1, 2, false, true)),
        ],
    );
    reject(&abi, &layouts, &wrong_mode);
}

#[test]
fn control_model_immutable_handle_graph_and_recursive_release_are_exact() {
    let (abi, layouts, _) = authorities();
    let shared = layouts
        .types()
        .find(|record| {
            record.category() == TypeCategory::Shared
                && record.referenced_type() == Some(ty(&layouts, TypeCategory::String))
        })
        .expect("Shared String")
        .id();
    let weak = ty(&layouts, TypeCategory::Weak);
    let structure = ty(&layouts, TypeCategory::Struct);
    let nodes = vec![
        PayloadNode { ty: shared, kind: PayloadKind::Handle(0) },
        PayloadNode { ty: weak, kind: PayloadKind::Handle(1) },
        PayloadNode { ty: structure, kind: PayloadKind::Children(vec![0, 1]) },
    ];
    let events = vec![
        seed(&abi, &layouts),
        handle(LogicalOperation::WeakDowngrade, 0, Some(1), None, state(1, 2, false, true)),
        construct(&abi, &layouts, nodes, 1, 2, 2048),
        handle(LogicalOperation::StrongReleaseBegin, 2, None, Some(true), state(0, 1, true, true)),
        handle(LogicalOperation::WeakRelease, 1, None, Some(false), state(1, 1, false, true)),
        handle(LogicalOperation::StrongReleaseBegin, 0, None, Some(true), state(0, 1, true, true)),
        ControlEventKind::DropPayloadNode { control: 0, node: 0 },
        ControlEventKind::Finish { control: 0, after: state(0, 0, false, false) },
        ControlEventKind::Finish { control: 1, after: state(0, 0, false, false) },
    ];
    let valid = trace(&abi, &layouts, events);
    let proof = verify_control_trace(&abi, &layouts, 17, &[], &valid)
        .expect("older issued handles only; nested release");
    assert_eq!(proof.live_owners(), 0);
    for mutation in 0..3 {
        let mut bad = valid.clone();
        let ControlEventKind::Construct { nodes, .. } = &mut bad.events[2].kind else {
            unreachable!()
        };
        match mutation {
            0 => nodes[1].kind = PayloadKind::Handle(2),
            1 => nodes[2].kind = PayloadKind::Children(vec![0, 0]),
            2 => bad.events.swap(4, 5),
            _ => unreachable!(),
        }
        for (site, event) in bad.events.iter_mut().enumerate() {
            event.site = site as u64;
        }
        reject(&abi, &layouts, &bad);
    }
}
