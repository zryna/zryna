use zryna_layout::{StorageTarget, TypeCategory, raw as raw_layout};
use zryna_source::{SourceFileInput, SourceMap};

use serde::Deserialize;

use super::{
    CONTROL_FORMULAS, ControlState, LogicalOperation, MAX_ALLOCATION_OPERATIONS, MAX_CALL_EDGES,
    MAX_CHECKED_HEADER_BYTES, MAX_DECLARATION_CHILDREN, MAX_DYNAMIC_ALLOCATION_BYTES,
    MAX_LAYOUT_REFERENCES, MAX_LIVE_ALLOCATIONS, MAX_RUNTIME_ARTIFACT_BYTES,
    MAX_RUNTIME_OPERATIONS, MAX_STATUS_TRANSITIONS, MAX_STRING_BYTES, MAX_TARGET_FUNCTIONS,
    MAX_VEC_ELEMENTS, MAX_VIOLATIONS, NON_CAPABILITIES, OWNERSHIP_RUNTIME_V1_IDENTIFIER,
    OWNERSHIP_RUNTIME_V1_SCHEMA_VERSION, RuntimeAbiViolationKind, RuntimeStatus,
    TRANSITION_RULE_IDENTITIES, TransitionClaim, raw, raw_v1, validate_transition, verify_v1,
};

const CHECKED_FIXTURE: &str = include_str!("../../../spec/abi/ownership-runtime-v1-fixtures.json");

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Fixture {
    schema_version: u32,
    abi_id: String,
    statuses: Vec<FixtureStatus>,
    operation_order: Vec<String>,
    operations: Vec<FixtureOperation>,
    records: Vec<FixtureRecord>,
    control_block: FixtureControlBlock,
    limits: FixtureLimits,
    transition_cases: Vec<FixtureTransition>,
    non_capabilities: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FixtureStatus {
    number: u8,
    name: String,
    disposition: String,
    trap_identity: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FixtureOperation {
    name: String,
    javascript: FixtureTarget,
    web_assembly: FixtureTarget,
    native_linux_x86_64: FixtureNativeTarget,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FixtureTarget {
    parameters: Vec<FixtureParameter>,
    result: FixtureResult,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FixtureNativeTarget {
    symbol: String,
    parameters: Vec<FixtureParameter>,
    result: FixtureResult,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FixtureParameter {
    name: String,
    role: String,
    carrier: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FixtureResult {
    carrier: String,
    out_record: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FixtureRecord {
    target: String,
    kind: String,
    size: u64,
    alignment: u64,
    fields: Vec<FixtureField>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FixtureField {
    name: String,
    offset: u64,
    carrier: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FixtureControlBlock {
    endianness: String,
    prefix_size: u64,
    strong_count: FixtureCountField,
    weak_count: FixtureCountField,
    payload_offset: String,
    control_alignment: String,
    control_size: String,
    weak_count_rule: String,
    pending_last_strong_rule: String,
    zero_sized_payload_uses_nonzero_allocation: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FixtureCountField {
    offset: u64,
    carrier: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FixtureLimits {
    dynamic_allocation_bytes: u64,
    string_bytes: u64,
    vec_elements: u64,
    allocation_alignments: Vec<u64>,
    strong_handle_count: u64,
    weak_count: u64,
    live_allocations_per_invocation: u64,
    allocation_growth_operations_per_invocation: u64,
    runtime_status_transitions_per_invocation: u64,
    wasm_memory_minimum_pages: u64,
    wasm_memory_maximum_pages: u64,
    wasm_heap_alignment_bytes: u64,
    runtime_operations: usize,
    runtime_symbols: usize,
    runtime_layout_references: usize,
    runtime_edges: usize,
    runtime_object_bytes: usize,
    diagnostics: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FixtureTransition {
    id: String,
    operation: String,
    before: FixtureControlState,
    result: FixtureTransitionResult,
    after: FixtureControlState,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FixtureControlState {
    strong_count: u32,
    weak_count: u32,
    pending_last_strong: bool,
    payload_initialized: bool,
    allocated: bool,
}

impl From<FixtureControlState> for ControlState {
    fn from(value: FixtureControlState) -> Self {
        Self {
            strong_count: value.strong_count,
            weak_count: value.weak_count,
            pending_last_strong: value.pending_last_strong,
            payload_initialized: value.payload_initialized,
            allocated: value.allocated,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FixtureTransitionResult {
    status: String,
    bool_result: Option<bool>,
}

fn authorities() -> (SourceMap, zryna_layout::VerifiedLayouts, zryna_layout::VerifiedLayouts) {
    let sources = SourceMap::build(vec![SourceFileInput {
        path: "src/main.zry".to_owned(),
        text: "runtime layout universe\n".to_owned(),
    }])
    .expect("source map");
    let file = sources.verify_file_id(0).expect("file");
    let graph = raw_layout::Graph {
        modules: vec![raw_layout::Module {
            id: raw_layout::ModuleId(0),
            source_file: file,
            data_declarations: 0,
        }],
        types: vec![
            raw_layout::TypeNode {
                id: raw_layout::NodeId(0),
                span: None,
                kind: raw_layout::TypeKind::Bool,
            },
            raw_layout::TypeNode {
                id: raw_layout::NodeId(1),
                span: None,
                kind: raw_layout::TypeKind::I32,
            },
            raw_layout::TypeNode {
                id: raw_layout::NodeId(2),
                span: None,
                kind: raw_layout::TypeKind::String,
            },
            raw_layout::TypeNode {
                id: raw_layout::NodeId(3),
                span: None,
                kind: raw_layout::TypeKind::Vec { element: raw_layout::NodeId(1) },
            },
            raw_layout::TypeNode {
                id: raw_layout::NodeId(4),
                span: None,
                kind: raw_layout::TypeKind::Shared { payload: raw_layout::NodeId(1) },
            },
            raw_layout::TypeNode {
                id: raw_layout::NodeId(5),
                span: None,
                kind: raw_layout::TypeKind::Weak { payload: raw_layout::NodeId(1) },
            },
        ],
        program_roots: (0..6).map(raw_layout::NodeId).collect(),
    };
    let linear =
        zryna_layout::verify(&graph, &sources, StorageTarget::Linear32V1).expect("linear layouts");
    let linux =
        zryna_layout::verify(&graph, &sources, StorageTarget::LinuxX8664V1).expect("linux layouts");
    (sources, linear, linux)
}

fn parse_fixture(text: &str) -> Result<Fixture, serde_json::Error> {
    serde_json::from_str(text)
}

fn fixture_status(value: &str) -> Option<RuntimeStatus> {
    Some(match value {
        "OK" => RuntimeStatus::Ok,
        "ALLOCATION" => RuntimeStatus::Allocation,
        "CAPACITY" => RuntimeStatus::Capacity,
        "REFCOUNT" => RuntimeStatus::Refcount,
        "UTF8" => RuntimeStatus::Utf8,
        "EXPIRED" => RuntimeStatus::Expired,
        "ABI_VIOLATION" => RuntimeStatus::AbiViolation,
        _ => return None,
    })
}

fn fixture_operation(value: &str) -> Option<LogicalOperation> {
    super::OPERATIONS.iter().copied().find(|operation| operation.name() == value)
}

fn fixture_parameter_names(operation: LogicalOperation, target: &str) -> &'static [&'static str] {
    match (operation, target) {
        (LogicalOperation::Allocate, "logical" | "wasm") => &["byteSize", "alignment"],
        (LogicalOperation::Grow, "logical" | "wasm") => {
            &["pointer", "oldByteSize", "newByteSize", "alignment"]
        }
        (LogicalOperation::Release, "logical" | "wasm") => &["pointer", "byteSize", "alignment"],
        (LogicalOperation::StringFromUtf8Copy, "logical") => &["bytes", "byteLength"],
        (LogicalOperation::StringClone, "logical") => &["source"],
        (LogicalOperation::StringConcat, "logical") => &["left", "right"],
        (LogicalOperation::StringRelease, "logical") => &["value"],
        (LogicalOperation::VecAllocate, "logical") => &["elementLayout", "requiredCapacity"],
        (LogicalOperation::VecReserve, "logical") => {
            &["elementLayout", "storage", "requiredLength"]
        }
        (LogicalOperation::VecReleaseStorage, "logical") => &["elementLayout", "storage"],
        (LogicalOperation::StringFromUtf8Copy, "wasm") => &["bytes", "byteLength", "outString"],
        (LogicalOperation::StringClone, "wasm") => {
            &["pointer", "byteLength", "capacity", "outString"]
        }
        (LogicalOperation::StringConcat, "wasm") => &[
            "leftPointer",
            "leftLength",
            "leftCapacity",
            "rightPointer",
            "rightLength",
            "rightCapacity",
            "outString",
        ],
        (LogicalOperation::StringRelease, "wasm") => &["pointer", "byteLength", "capacity"],
        (LogicalOperation::VecAllocate, "wasm") => {
            &["elementLayoutId", "requiredCapacity", "outStorage"]
        }
        (LogicalOperation::VecReserve, "wasm") => &[
            "elementLayoutId",
            "pointer",
            "elementLength",
            "capacity",
            "requiredLength",
            "outStorage",
        ],
        (LogicalOperation::VecReleaseStorage, "wasm") => {
            &["elementLayoutId", "pointer", "elementLength", "capacity"]
        }
        (LogicalOperation::StrongReleaseBegin, "wasm") => &["control", "outIsLastStrong"],
        (LogicalOperation::WeakRelease, "wasm") => &["control", "outDeallocated"],
        (_, "logical" | "wasm") => &["control"],
        _ => &[],
    }
}

fn logical_carrier(parameter: raw::LogicalParameter) -> &'static str {
    match parameter {
        raw::LogicalParameter::ByteSize
        | raw::LogicalParameter::OldByteSize
        | raw::LogicalParameter::NewByteSize
        | raw::LogicalParameter::ByteLength
        | raw::LogicalParameter::RequiredCapacity
        | raw::LogicalParameter::RequiredLength => "unsigned-integer",
        raw::LogicalParameter::Alignment => "alignment",
        raw::LogicalParameter::Pointer => "opaque-pointer",
        raw::LogicalParameter::Bytes => "byte-sequence",
        raw::LogicalParameter::String
        | raw::LogicalParameter::LeftString
        | raw::LogicalParameter::RightString => "string-handle",
        raw::LogicalParameter::ElementLayout => "sealed-layout",
        raw::LogicalParameter::VecStorage => "vec-storage",
        raw::LogicalParameter::Control => "control-handle",
    }
}

fn logical_role(parameter: raw::LogicalParameter) -> &'static str {
    match parameter {
        raw::LogicalParameter::Bytes
        | raw::LogicalParameter::String
        | raw::LogicalParameter::LeftString
        | raw::LogicalParameter::RightString
        | raw::LogicalParameter::ElementLayout
        | raw::LogicalParameter::VecStorage => "const-input",
        _ => "input",
    }
}

fn native_fixture_carrier(carrier: raw::NativeCarrier) -> &'static str {
    match carrier {
        raw::NativeCarrier::U32 => "uint32_t",
        raw::NativeCarrier::U64 => "uint64_t",
        raw::NativeCarrier::UintPtr => "uintptr_t",
        raw::NativeCarrier::ConstU8Pointer => "const uint8_t *",
        raw::NativeCarrier::ConstHandlePointer => "const zryna_rt_o1_handle *",
        raw::NativeCarrier::MutHandlePointer => "zryna_rt_o1_handle *",
        raw::NativeCarrier::MutUintPtrPointer => "uintptr_t *",
        raw::NativeCarrier::MutU32Pointer => "uint32_t *",
    }
}

fn native_role(carrier: raw::NativeCarrier) -> &'static str {
    match carrier {
        raw::NativeCarrier::ConstU8Pointer | raw::NativeCarrier::ConstHandlePointer => {
            "const-input"
        }
        raw::NativeCarrier::MutHandlePointer
        | raw::NativeCarrier::MutUintPtrPointer
        | raw::NativeCarrier::MutU32Pointer => "out-zeroed",
        _ => "input",
    }
}

fn authenticate_fixture(fixture: &Fixture, canonical: &raw::Contract) -> Result<(), String> {
    if fixture.schema_version != canonical.schema_version
        || fixture.schema_version != OWNERSHIP_RUNTIME_V1_SCHEMA_VERSION
        || fixture.abi_id != canonical.identifier
        || fixture.abi_id != OWNERSHIP_RUNTIME_V1_IDENTIFIER
    {
        return Err("fixture identity/version mismatch".to_owned());
    }
    if fixture.statuses.len() != canonical.statuses.len() {
        return Err("fixture status count mismatch".to_owned());
    }
    for (fixture_status, raw_status) in fixture.statuses.iter().zip(&canonical.statuses) {
        let disposition = match raw_status.disposition {
            raw::StatusDisposition::Success => "success",
            raw::StatusDisposition::ControlledTrap => "controlled-trap",
            raw::StatusDisposition::Branch => "branch",
            raw::StatusDisposition::HostFailure => "host-failure",
        };
        if fixture_status.number != raw_status.numeric
            || fixture_status.name != raw_status.name
            || fixture_status.disposition != disposition
            || fixture_status.trap_identity != raw_status.trap_identity
        {
            return Err("fixture status mismatch".to_owned());
        }
    }
    let raw_order = canonical.operations.iter().map(|row| row.name.as_str()).collect::<Vec<_>>();
    if fixture.operation_order.iter().map(String::as_str).collect::<Vec<_>>() != raw_order
        || fixture.operations.len() != canonical.operations.len()
    {
        return Err("fixture operation inventory mismatch".to_owned());
    }
    for (index, fixture_row) in fixture.operations.iter().enumerate() {
        authenticate_operation(fixture_row, canonical, index)?;
    }
    authenticate_records(&fixture.records, canonical)?;
    authenticate_control_block(&fixture.control_block)?;
    authenticate_limits(&fixture.limits)?;
    if fixture.transition_cases.len() != TRANSITION_RULE_IDENTITIES.len() {
        return Err("fixture transition inventory mismatch".to_owned());
    }
    for (index, transition) in fixture.transition_cases.iter().enumerate() {
        if transition.id != TRANSITION_RULE_IDENTITIES[index] {
            return Err("fixture transition order mismatch".to_owned());
        }
        let operation = fixture_operation(&transition.operation)
            .ok_or_else(|| "fixture transition operation mismatch".to_owned())?;
        let status = fixture_status(&transition.result.status)
            .ok_or_else(|| "fixture transition status mismatch".to_owned())?;
        validate_transition(TransitionClaim::Control {
            operation,
            before: transition.before.into(),
            status,
            bool_result: transition.result.bool_result,
            after: transition.after.into(),
        })
        .map_err(|_| format!("fixture transition {} is invalid", transition.id))?;
    }
    if fixture.non_capabilities.iter().map(String::as_str).collect::<Vec<_>>() != NON_CAPABILITIES {
        return Err("fixture non-capability inventory mismatch".to_owned());
    }
    Ok(())
}

#[allow(clippy::too_many_lines)]
fn authenticate_operation(
    fixture: &FixtureOperation,
    canonical: &raw::Contract,
    index: usize,
) -> Result<(), String> {
    let logical = canonical.operations.get(index).ok_or_else(|| "operation missing".to_owned())?;
    let operation =
        fixture_operation(&fixture.name).ok_or_else(|| "unknown fixture operation".to_owned())?;
    if fixture.name != logical.name {
        return Err("operation association mismatch".to_owned());
    }
    let names = fixture_parameter_names(operation, "logical");
    if fixture.javascript.parameters.len() != logical.parameters.len()
        || names.len() != logical.parameters.len()
    {
        return Err("JavaScript parameter count mismatch".to_owned());
    }
    for ((parameter, raw_parameter), name) in
        fixture.javascript.parameters.iter().zip(&logical.parameters).zip(names)
    {
        if parameter.name != *name
            || parameter.role != logical_role(*raw_parameter)
            || parameter.carrier != logical_carrier(*raw_parameter)
        {
            return Err("JavaScript logical parameter mismatch".to_owned());
        }
    }
    let (js_carrier, js_record) = match logical.result {
        raw::LogicalResult::Status => ("numeric-status", None),
        raw::LogicalResult::StatusPointer => ("numeric-status", Some("pointer")),
        raw::LogicalResult::StatusString | raw::LogicalResult::StatusVecStorage => {
            ("numeric-status", Some("handle"))
        }
        raw::LogicalResult::StatusBool => ("numeric-status", Some("bool")),
    };
    if fixture.javascript.result.carrier != js_carrier
        || fixture.javascript.result.out_record.as_deref() != js_record
    {
        return Err("JavaScript result mismatch".to_owned());
    }
    let wasm = &canonical.webassembly[index];
    let wasm_names = fixture_parameter_names(operation, "wasm");
    if fixture.web_assembly.parameters.len() != wasm.parameters.len()
        || wasm_names.len() != wasm.parameters.len()
    {
        return Err("WebAssembly parameter count mismatch".to_owned());
    }
    for ((parameter, lane), name) in
        fixture.web_assembly.parameters.iter().zip(&wasm.parameters).zip(wasm_names)
    {
        let expected_carrier = match lane {
            raw::WebAssemblyLane::I32 => "i32",
            raw::WebAssemblyLane::I64 => "i64",
        };
        let expected_role = if name.starts_with("out") {
            "out-zeroed"
        } else if *name == "elementLayoutId"
            || (matches!(*name, "bytes" | "pointer" | "leftPointer" | "rightPointer")
                && matches!(
                    operation,
                    LogicalOperation::StringFromUtf8Copy
                        | LogicalOperation::StringClone
                        | LogicalOperation::StringConcat
                        | LogicalOperation::StringRelease
                        | LogicalOperation::VecReserve
                        | LogicalOperation::VecReleaseStorage
                ))
        {
            "const-input"
        } else {
            "input"
        };
        if parameter.name != *name
            || parameter.role != expected_role
            || parameter.carrier != expected_carrier
        {
            return Err("WebAssembly parameter mismatch".to_owned());
        }
    }
    let wasm_carrier = match wasm.results.as_slice() {
        [raw::WebAssemblyLane::I64] => "i64-packed-status-pointer",
        [raw::WebAssemblyLane::I32] => "i32-status",
        _ => return Err("WebAssembly result lanes mismatch".to_owned()),
    };
    let wasm_record =
        matches!(wasm.results.as_slice(), [raw::WebAssemblyLane::I64]).then_some("pointer");
    if fixture.web_assembly.result.carrier != wasm_carrier
        || fixture.web_assembly.result.out_record.as_deref() != wasm_record
    {
        return Err("WebAssembly result mismatch".to_owned());
    }
    let native = &canonical.native_linux_x86_64[index];
    let native_names = super::native_parameter_names(operation);
    if fixture.native_linux_x86_64.symbol != native.symbol
        || fixture.native_linux_x86_64.parameters.len() != native.parameters.len()
        || native_names.len() != native.parameters.len()
    {
        return Err("native declaration mismatch".to_owned());
    }
    for ((parameter, carrier), name) in
        fixture.native_linux_x86_64.parameters.iter().zip(&native.parameters).zip(native_names)
    {
        let expected_role =
            if *name == "element_layout_id" { "const-input" } else { native_role(*carrier) };
        let fixture_name =
            name.split('_').enumerate().fold(String::new(), |mut value, (part, word)| {
                if part == 0 {
                    value.push_str(word);
                } else {
                    let mut chars = word.chars();
                    if let Some(first) = chars.next() {
                        value.push(first.to_ascii_uppercase());
                    }
                    value.extend(chars);
                }
                value
            });
        if parameter.name != fixture_name
            || parameter.role != expected_role
            || parameter.carrier != native_fixture_carrier(*carrier)
        {
            return Err(format!(
                "native parameter mismatch for {}: {:?} != ({fixture_name}, {}, {})",
                fixture.name,
                (parameter.name.as_str(), parameter.role.as_str(), parameter.carrier.as_str()),
                expected_role,
                native_fixture_carrier(*carrier)
            ));
        }
    }
    if fixture.native_linux_x86_64.result.carrier != "u32-status"
        || fixture.native_linux_x86_64.result.out_record.is_some()
        || native.result != raw::NativeCarrier::U32
    {
        return Err("native result mismatch".to_owned());
    }
    Ok(())
}

fn authenticate_records(
    fixture: &[FixtureRecord],
    canonical: &raw::Contract,
) -> Result<(), String> {
    if fixture.len() != 4 {
        return Err("fixture record inventory mismatch".to_owned());
    }
    let expected_inventory = [
        ("linear32-v1", "handle"),
        ("linear32-v1", "bool"),
        ("linux-x86-64-v1", "handle"),
        ("linux-x86-64-v1", "bool"),
    ];
    for (record, (expected_target, expected_kind)) in fixture.iter().zip(expected_inventory) {
        if record.target != expected_target || record.kind != expected_kind {
            return Err("fixture record inventory/order mismatch".to_owned());
        }
        let (target, word) = match record.target.as_str() {
            "linear32-v1" => (raw::RecordTarget::Linear32V1, 4_u64),
            "linux-x86-64-v1" => (raw::RecordTarget::LinuxX8664V1, 8_u64),
            _ => return Err("fixture record target mismatch".to_owned()),
        };
        match record.kind.as_str() {
            "handle" => {
                let fields = [("pointer", 0), ("length", word), ("capacity", word * 2)];
                if record.size != word * 3
                    || record.alignment != word
                    || record.fields.len() != fields.len()
                {
                    return Err("fixture handle record mismatch".to_owned());
                }
                for (actual, (name, offset)) in record.fields.iter().zip(fields) {
                    let carrier = if word == 4 {
                        "u32"
                    } else if name == "pointer" {
                        "uintptr_t"
                    } else {
                        "uint64_t"
                    };
                    if actual.name != name || actual.offset != offset || actual.carrier != carrier {
                        return Err("fixture handle field mismatch".to_owned());
                    }
                }
                for kind in [raw::RecordKind::StringHandle, raw::RecordKind::VecHandle] {
                    if !canonical.records.iter().any(|item| {
                        item.target == target
                            && item.kind == kind
                            && item.size == record.size
                            && item.alignment == record.alignment
                    }) {
                        return Err("fixture handle is not bound to canonical records".to_owned());
                    }
                }
            }
            "bool" => {
                let carrier = if word == 4 && target == raw::RecordTarget::Linear32V1 {
                    "u32-zero-or-one"
                } else {
                    "uint32_t-zero-or-one"
                };
                if record.size != 4
                    || record.alignment != 4
                    || record.fields.len() != 1
                    || record.fields[0].name != "value"
                    || record.fields[0].offset != 0
                    || record.fields[0].carrier != carrier
                    || !canonical.records.iter().any(|item| {
                        item.target == target
                            && item.kind == raw::RecordKind::BoolOutcome
                            && item.size == 4
                            && item.alignment == 4
                    })
                {
                    return Err("fixture bool record mismatch".to_owned());
                }
            }
            _ => return Err("fixture record kind mismatch".to_owned()),
        }
    }
    Ok(())
}

fn authenticate_control_block(control: &FixtureControlBlock) -> Result<(), String> {
    let actual = [
        format!("endianness={}", control.endianness),
        format!("prefix-size={}", control.prefix_size),
        format!("payload-offset={}", control.payload_offset),
        format!("control-alignment={}", control.control_alignment),
        format!("control-size={}", control.control_size),
        format!("weak-count={}", control.weak_count_rule),
        format!("pending-last-strong={}", control.pending_last_strong_rule),
        format!(
            "zero-sized-payload-uses-nonzero-allocation={}",
            control.zero_sized_payload_uses_nonzero_allocation
        ),
    ];
    if actual.iter().map(String::as_str).collect::<Vec<_>>() != CONTROL_FORMULAS
        || control.strong_count.offset != 0
        || control.strong_count.carrier != "u32"
        || control.weak_count.offset != 4
        || control.weak_count.carrier != "u32"
        || !control.zero_sized_payload_uses_nonzero_allocation
    {
        return Err("fixture control formula mismatch".to_owned());
    }
    Ok(())
}

fn authenticate_limits(limits: &FixtureLimits) -> Result<(), String> {
    if limits.dynamic_allocation_bytes != MAX_DYNAMIC_ALLOCATION_BYTES
        || limits.string_bytes != MAX_STRING_BYTES
        || limits.vec_elements != MAX_VEC_ELEMENTS
        || limits.allocation_alignments != [1, 2, 4, 8]
        || limits.strong_handle_count != u64::from(u32::MAX)
        || limits.weak_count != u64::from(u32::MAX)
        || limits.live_allocations_per_invocation != MAX_LIVE_ALLOCATIONS
        || limits.allocation_growth_operations_per_invocation != MAX_ALLOCATION_OPERATIONS
        || limits.runtime_status_transitions_per_invocation != MAX_STATUS_TRANSITIONS
        || limits.wasm_memory_minimum_pages != 1
        || limits.wasm_memory_maximum_pages != 32_768
        || limits.wasm_heap_alignment_bytes != 8
        || limits.runtime_operations != MAX_RUNTIME_OPERATIONS
        || limits.runtime_symbols != MAX_TARGET_FUNCTIONS
        || limits.runtime_layout_references != MAX_LAYOUT_REFERENCES
        || limits.runtime_edges != MAX_CALL_EDGES
        || limits.runtime_object_bytes != MAX_RUNTIME_ARTIFACT_BYTES
        || limits.diagnostics != MAX_VIOLATIONS
    {
        return Err("fixture limit inventory mismatch".to_owned());
    }
    Ok(())
}

#[test]
fn exact_contract_seals_all_declarations_and_layout_metadata() {
    let (_sources, linear, linux) = authorities();
    let raw = raw_v1(&linear, &linux);
    assert_eq!(raw.statuses.len(), 7);
    assert_eq!(raw.operations.len(), 17);
    assert_eq!(raw.javascript.len(), 17);
    assert_eq!(raw.webassembly.len(), 17);
    assert_eq!(raw.native_linux_x86_64.len(), 17);
    assert_eq!(raw.operations[0].name, "allocate");
    assert_eq!(raw.operations[16].name, "weakRelease");
    assert!(raw.javascript.iter().all(|helper| !helper.operation.starts_with("zryna_rt_")));

    let verified = verify_v1(raw, &linear, &linux).expect("exact runtime ABI");
    assert_eq!(verified.identifier(), "zryna-ownership-runtime-v1");
    assert_eq!(verified.statuses().len(), 7);
    assert_eq!(verified.operations().len(), 17);
    assert_eq!(verified.javascript_helpers().len(), 17);
    assert_eq!(verified.webassembly_functions().len(), 17);
    assert_eq!(verified.native_linux_x86_64_functions().len(), 17);
    assert!(verified.records().len() >= 6);
    assert_eq!(
        verified.native_linux_x86_64_functions().next().expect("allocate").symbol(),
        "zryna_rt_o1_allocate"
    );
    assert_eq!(
        verified.operations().map(super::VerifiedOperation::operation).collect::<Vec<_>>(),
        [
            LogicalOperation::Allocate,
            LogicalOperation::Grow,
            LogicalOperation::Release,
            LogicalOperation::StringFromUtf8Copy,
            LogicalOperation::StringClone,
            LogicalOperation::StringConcat,
            LogicalOperation::StringRelease,
            LogicalOperation::VecAllocate,
            LogicalOperation::VecReserve,
            LogicalOperation::VecReleaseStorage,
            LogicalOperation::StrongClone,
            LogicalOperation::WeakDowngrade,
            LogicalOperation::WeakClone,
            LogicalOperation::WeakUpgrade,
            LogicalOperation::StrongReleaseBegin,
            LogicalOperation::StrongReleaseFinish,
            LogicalOperation::WeakRelease,
        ]
    );
    let elements = verified.element_layouts().collect::<Vec<_>>();
    assert_eq!(elements.len(), 2);
    assert!(elements.iter().all(|element| element.stride() == 4 && element.alignment() == 4));
    let controls = verified.control_layouts().collect::<Vec<_>>();
    assert_eq!(controls.len(), 2);
    assert!(controls.iter().all(|control| {
        control.payload_offset() == 8 && control.size() == 12 && control.alignment() == 4
    }));
    assert_eq!(verified.native_linux_x86_64_header(), raw_v1(&linear, &linux).native_header);
    assert_ne!(verified.identity().as_bytes(), [0; 32]);
}

#[test]
fn exact_authority_is_deterministic() {
    let (_sources, linear, linux) = authorities();
    let first = verify_v1(raw_v1(&linear, &linux), &linear, &linux).expect("first");
    let second = verify_v1(raw_v1(&linear, &linux), &linear, &linux).expect("second");
    assert_eq!(first.identity(), second.identity());
    assert_eq!(first.linear32_fingerprint(), *linear.fingerprint());
    assert_eq!(first.linux_x86_64_fingerprint(), *linux.fingerprint());
}

#[test]
fn authority_fingerprint_is_domain_separated_and_binds_semantic_declarations() {
    let (_sources, linear, linux) = authorities();
    let canonical = raw_v1(&linear, &linux);
    let (elements, controls) = super::derive_metadata(&linear, &linux).expect("metadata");
    let identity = super::fingerprint(&linear, &linux, &canonical, &elements, &controls);

    let mutations: [fn(&mut raw::Contract); 4] = [
        |contract: &mut raw::Contract| contract.schema_version += 1,
        |contract: &mut raw::Contract| {
            contract.statuses[1].trap_identity = Some("different.trap".to_owned());
        },
        |contract: &mut raw::Contract| {
            contract.javascript[0].operation = "grow".to_owned();
        },
        |contract: &mut raw::Contract| contract.native_header.push(b'\n'),
    ];
    for mutate in mutations {
        let mut changed = canonical.clone();
        mutate(&mut changed);
        assert_ne!(identity, super::fingerprint(&linear, &linux, &changed, &elements, &controls));
    }
}

#[test]
fn inventory_status_mapping_record_and_header_forgeries_fail_closed() {
    let (_sources, linear, linux) = authorities();

    let mut identifier = raw_v1(&linear, &linux);
    identifier.identifier.push_str("-forged");
    assert_eq!(
        verify_v1(identifier, &linear, &linux).expect_err("identifier")[0].code(),
        "ZRYNA-R3001"
    );

    let mut status = raw_v1(&linear, &linux);
    status.statuses[1].numeric = 9;
    assert_eq!(verify_v1(status, &linear, &linux).expect_err("status")[0].code(), "ZRYNA-R3002");

    let mut reordered = raw_v1(&linear, &linux);
    reordered.operations.swap(0, 1);
    assert_eq!(verify_v1(reordered, &linear, &linux).expect_err("order")[0].code(), "ZRYNA-R3001");

    let mut mapping = raw_v1(&linear, &linux);
    mapping.webassembly[0].results.clear();
    assert_eq!(verify_v1(mapping, &linear, &linux).expect_err("mapping")[0].code(), "ZRYNA-R3002");

    let mut symbol = raw_v1(&linear, &linux);
    symbol.native_linux_x86_64[0].symbol = "allocate".to_owned();
    assert_eq!(verify_v1(symbol, &linear, &linux).expect_err("symbol")[0].code(), "ZRYNA-R3001");

    let mut record = raw_v1(&linear, &linux);
    record.records[0].size += 1;
    assert_eq!(verify_v1(record, &linear, &linux).expect_err("record")[0].code(), "ZRYNA-R3003");

    let mut header = raw_v1(&linear, &linux);
    header.native_header.push(b'\n');
    assert_eq!(verify_v1(header, &linear, &linux).expect_err("header")[0].code(), "ZRYNA-R3005");
}

#[test]
fn layout_binding_rejects_target_and_fingerprint_mismatch() {
    let (_sources, linear, linux) = authorities();
    let raw = raw_v1(&linear, &linux);
    assert_eq!(
        verify_v1(raw.clone(), &linux, &linear).expect_err("target tuple")[0].code(),
        "ZRYNA-R3003"
    );
    let mut fingerprint = raw;
    fingerprint.layout_claims[0].fingerprint[0] ^= 1;
    assert_eq!(
        verify_v1(fingerprint, &linear, &linux).expect_err("fingerprint")[0].code(),
        "ZRYNA-R3003"
    );
}

#[test]
fn untrusted_operation_preflight_limit_is_exact_and_plus_one_fails() {
    let (_sources, linear, linux) = authorities();
    let seed = raw_v1(&linear, &linux);
    let mut exact = seed.clone();
    exact.operations = vec![seed.operations[0].clone(); MAX_RUNTIME_OPERATIONS];
    let exact_errors = verify_v1(exact, &linear, &linux).expect_err("inventory is not exact");
    assert!(exact_errors.iter().all(|error| error.code() != "ZRYNA-R3201"));

    let mut plus = seed;
    plus.operations = vec![plus.operations[0].clone(); MAX_RUNTIME_OPERATIONS + 1];
    let diagnostics = verify_v1(plus, &linear, &linux).expect_err("preflight plus one");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-R3201");
}

#[test]
fn violations_are_bounded_and_deterministic() {
    let (_sources, linear, linux) = authorities();
    let mut raw = raw_v1(&linear, &linux);
    raw.operations[0].name = "forged".to_owned();
    raw.operations = vec![raw.operations[0].clone(); MAX_RUNTIME_OPERATIONS];
    let first = verify_v1(raw.clone(), &linear, &linux).expect_err("first");
    let second = verify_v1(raw, &linear, &linux).expect_err("second");
    assert_eq!(first, second);
    assert_eq!(first.len(), 256);
    assert_eq!(first.last().expect("terminal").kind, RuntimeAbiViolationKind::Budget);
}

#[test]
fn pure_vec_and_count_transitions_are_exact() {
    validate_transition(TransitionClaim::VecAllocate {
        requested: 0,
        status: RuntimeStatus::Ok,
        pointer: 0,
        length: 0,
        capacity: 0,
    })
    .expect("canonical empty");
    validate_transition(TransitionClaim::VecAllocate {
        requested: 7,
        status: RuntimeStatus::Ok,
        pointer: 1,
        length: 0,
        capacity: 7,
    })
    .expect("fresh capacity equals request");
    assert!(
        validate_transition(TransitionClaim::VecAllocate {
            requested: 7,
            status: RuntimeStatus::Ok,
            pointer: 1,
            length: 0,
            capacity: 8,
        })
        .is_err()
    );
    validate_transition(TransitionClaim::VecReserve {
        old_length: 3,
        old_capacity: 4,
        required_length: 9,
        status: RuntimeStatus::Ok,
        pointer: 9,
        length: 3,
        capacity: 16,
        input_unchanged: false,
    })
    .expect("frozen doubling");
    validate_transition(TransitionClaim::CountIncrement {
        before: 7,
        status: RuntimeStatus::Ok,
        after: 8,
    })
    .expect("count increment");
    validate_transition(TransitionClaim::CountIncrement {
        before: u32::MAX,
        status: RuntimeStatus::Refcount,
        after: u32::MAX,
    })
    .expect("count overflow is atomic");
    validate_transition(TransitionClaim::WeakUpgrade {
        before: 0,
        status: RuntimeStatus::Expired,
        after: 0,
    })
    .expect("expired upgrade has no mutation");
    validate_transition(TransitionClaim::StrongReleaseBegin {
        before: 1,
        status: RuntimeStatus::Ok,
        after: 0,
        is_last: true,
    })
    .expect("last strong is exact");
    validate_transition(TransitionClaim::WeakRelease {
        before: 1,
        strong: 0,
        status: RuntimeStatus::Ok,
        after: 0,
        deallocated: true,
    })
    .expect("last weak deallocates only at strong zero");
    validate_transition(TransitionClaim::FailureAtomic {
        status: RuntimeStatus::Allocation,
        outputs_zero: true,
        input_unchanged: true,
    })
    .expect("failure atomicity");
}

#[test]
fn layout_categories_are_present_in_fixture() {
    let (_sources, linear, _linux) = authorities();
    let categories = linear.types().map(zryna_layout::VerifiedType::category).collect::<Vec<_>>();
    assert!(categories.contains(&TypeCategory::String));
    assert!(categories.contains(&TypeCategory::Vec));
    assert!(categories.contains(&TypeCategory::Shared));
    assert!(categories.contains(&TypeCategory::Weak));
}

#[test]
fn checked_json_fixture_authenticates_every_inventory() {
    let fixture = parse_fixture(CHECKED_FIXTURE).expect("strict fixture decode");
    let (_sources, linear, linux) = authorities();
    authenticate_fixture(&fixture, &raw_v1(&linear, &linux)).expect("fixture authority");
}

#[test]
fn checked_json_fixture_rejects_unknown_duplicate_deleted_reordered_and_drifted_fields() {
    let unknown = CHECKED_FIXTURE.replacen('{', "{\"unknown\":true,", 1);
    assert!(parse_fixture(&unknown).is_err());
    let duplicate = CHECKED_FIXTURE.replacen(
        "\"schemaVersion\": 1,",
        "\"schemaVersion\": 1,\"schemaVersion\": 1,",
        1,
    );
    assert!(parse_fixture(&duplicate).is_err());

    let (_sources, linear, linux) = authorities();
    let canonical = raw_v1(&linear, &linux);
    let mut deleted = parse_fixture(CHECKED_FIXTURE).expect("fixture");
    deleted.operations.pop();
    assert!(authenticate_fixture(&deleted, &canonical).is_err());
    let mut reordered = parse_fixture(CHECKED_FIXTURE).expect("fixture");
    reordered.operations.swap(0, 1);
    assert!(authenticate_fixture(&reordered, &canonical).is_err());
    let mut duplicate = parse_fixture(CHECKED_FIXTURE).expect("fixture");
    let mut duplicate_row = parse_fixture(CHECKED_FIXTURE).expect("fixture");
    duplicate.operations[1] = duplicate_row.operations.remove(0);
    assert!(authenticate_fixture(&duplicate, &canonical).is_err());
    let mut extra = parse_fixture(CHECKED_FIXTURE).expect("fixture");
    let mut extra_row = parse_fixture(CHECKED_FIXTURE).expect("fixture");
    extra.operations.push(extra_row.operations.remove(0));
    assert!(authenticate_fixture(&extra, &canonical).is_err());
    let mut carrier = parse_fixture(CHECKED_FIXTURE).expect("fixture");
    carrier.operations[0].native_linux_x86_64.parameters[0].carrier = "size_t".to_owned();
    assert!(authenticate_fixture(&carrier, &canonical).is_err());
    let mut record = parse_fixture(CHECKED_FIXTURE).expect("fixture");
    record.records[0].fields[0].offset = 4;
    assert!(authenticate_fixture(&record, &canonical).is_err());
    let mut reordered_records = parse_fixture(CHECKED_FIXTURE).expect("fixture");
    reordered_records.records.swap(0, 1);
    assert!(authenticate_fixture(&reordered_records, &canonical).is_err());
    let mut duplicate_record = parse_fixture(CHECKED_FIXTURE).expect("fixture");
    let mut duplicate_record_source = parse_fixture(CHECKED_FIXTURE).expect("fixture");
    duplicate_record.records[1] = duplicate_record_source.records.remove(0);
    assert!(authenticate_fixture(&duplicate_record, &canonical).is_err());
    let mut transition = parse_fixture(CHECKED_FIXTURE).expect("fixture");
    transition.transition_cases[0].after.strong_count = 9;
    assert!(authenticate_fixture(&transition, &canonical).is_err());
    let mut capability = parse_fixture(CHECKED_FIXTURE).expect("fixture");
    capability.non_capabilities.pop();
    assert!(authenticate_fixture(&capability, &canonical).is_err());
}

#[test]
fn raw_schema_version_and_status_semantics_fail_closed() {
    let (_sources, linear, linux) = authorities();
    let mut version = raw_v1(&linear, &linux);
    version.schema_version += 1;
    assert_eq!(verify_v1(version, &linear, &linux).expect_err("version")[0].code(), "ZRYNA-R3001");

    let mut disposition = raw_v1(&linear, &linux);
    disposition.statuses[1].disposition = raw::StatusDisposition::Branch;
    assert_eq!(
        verify_v1(disposition, &linear, &linux).expect_err("disposition")[0].code(),
        "ZRYNA-R3002"
    );
    let mut trap = raw_v1(&linear, &linux);
    trap.statuses[1].trap_identity = None;
    assert_eq!(verify_v1(trap, &linear, &linux).expect_err("trap")[0].code(), "ZRYNA-R3002");
}

#[test]
fn all_control_transitions_and_illegal_variants_are_checked() {
    let fixture = parse_fixture(CHECKED_FIXTURE).expect("fixture");
    for transition in &fixture.transition_cases {
        validate_transition(TransitionClaim::Control {
            operation: fixture_operation(&transition.operation).expect("operation"),
            before: transition.before.into(),
            status: fixture_status(&transition.result.status).expect("status"),
            bool_result: transition.result.bool_result,
            after: transition.after.into(),
        })
        .unwrap_or_else(|_| panic!("canonical transition {}", transition.id));
    }
    let finish = &fixture.transition_cases[8];
    assert!(
        validate_transition(TransitionClaim::Control {
            operation: LogicalOperation::StrongReleaseFinish,
            before: finish.before.into(),
            status: RuntimeStatus::Ok,
            bool_result: None,
            after: ControlState { allocated: true, ..finish.after.into() },
        })
        .is_err()
    );
    assert!(!super::operation_accepts_status(LogicalOperation::WeakUpgrade, RuntimeStatus::Utf8));
}

#[test]
fn pending_last_strong_excludes_every_operation_except_finish() {
    let pending = ControlState {
        strong_count: 0,
        weak_count: 2,
        pending_last_strong: true,
        payload_initialized: false,
        allocated: true,
    };
    let forbidden = [
        (
            LogicalOperation::WeakClone,
            RuntimeStatus::Ok,
            None,
            ControlState { weak_count: 3, ..pending },
        ),
        (LogicalOperation::WeakUpgrade, RuntimeStatus::Expired, None, pending),
        (
            LogicalOperation::WeakRelease,
            RuntimeStatus::Ok,
            Some(false),
            ControlState { weak_count: 1, ..pending },
        ),
    ];
    for (operation, status, bool_result, after) in forbidden {
        assert!(
            validate_transition(TransitionClaim::Control {
                operation,
                before: pending,
                status,
                bool_result,
                after,
            })
            .is_err(),
            "{operation:?} must not run while last-strong release is pending"
        );
    }

    validate_transition(TransitionClaim::Control {
        operation: LogicalOperation::WeakRelease,
        before: pending,
        status: RuntimeStatus::AbiViolation,
        bool_result: Some(false),
        after: pending,
    })
    .expect("fail-closed rejection is unchanged and zero-shaped");
    assert!(
        validate_transition(TransitionClaim::Control {
            operation: LogicalOperation::WeakRelease,
            before: pending,
            status: RuntimeStatus::AbiViolation,
            bool_result: Some(true),
            after: pending,
        })
        .is_err()
    );
}

#[test]
fn non_success_control_results_are_zero_shaped() {
    let live = ControlState {
        strong_count: 2,
        weak_count: 1,
        pending_last_strong: false,
        payload_initialized: true,
        allocated: true,
    };
    let expired = ControlState {
        strong_count: 0,
        weak_count: 1,
        pending_last_strong: false,
        payload_initialized: false,
        allocated: true,
    };
    let saturated = ControlState { strong_count: u32::MAX, ..live };
    let malformed = [
        (LogicalOperation::StrongReleaseBegin, live, RuntimeStatus::AbiViolation),
        (LogicalOperation::WeakUpgrade, expired, RuntimeStatus::Expired),
        (LogicalOperation::StrongClone, saturated, RuntimeStatus::Refcount),
    ];
    for (operation, state, status) in malformed {
        assert!(
            validate_transition(TransitionClaim::Control {
                operation,
                before: state,
                status,
                bool_result: Some(true),
                after: state,
            })
            .is_err(),
            "{operation:?} must reject a nonzero failure output"
        );
    }

    validate_transition(TransitionClaim::Control {
        operation: LogicalOperation::StrongReleaseBegin,
        before: live,
        status: RuntimeStatus::AbiViolation,
        bool_result: Some(false),
        after: live,
    })
    .expect("boolean-out failure is canonically zero");
    validate_transition(TransitionClaim::Control {
        operation: LogicalOperation::WeakUpgrade,
        before: expired,
        status: RuntimeStatus::Expired,
        bool_result: None,
        after: expired,
    })
    .expect("status-only failure has no boolean lane");
}

#[test]
fn vec_failures_use_only_operation_specific_atomic_statuses() {
    validate_transition(TransitionClaim::VecAllocate {
        requested: MAX_VEC_ELEMENTS + 1,
        status: RuntimeStatus::Capacity,
        pointer: 0,
        length: 0,
        capacity: 0,
    })
    .expect("oversized allocation is capacity failure");
    assert!(
        validate_transition(TransitionClaim::VecAllocate {
            requested: MAX_VEC_ELEMENTS + 1,
            status: RuntimeStatus::Allocation,
            pointer: 0,
            length: 0,
            capacity: 0,
        })
        .is_err()
    );
    assert!(
        validate_transition(TransitionClaim::VecAllocate {
            requested: 1,
            status: RuntimeStatus::Expired,
            pointer: 0,
            length: 0,
            capacity: 0,
        })
        .is_err()
    );
    assert!(
        validate_transition(TransitionClaim::VecReserve {
            old_length: 1,
            old_capacity: 1,
            required_length: 2,
            status: RuntimeStatus::Utf8,
            pointer: 0,
            length: 0,
            capacity: 0,
            input_unchanged: true,
        })
        .is_err()
    );
    assert!(
        validate_transition(TransitionClaim::VecReserve {
            old_length: 1,
            old_capacity: 1,
            required_length: MAX_VEC_ELEMENTS + 1,
            status: RuntimeStatus::Allocation,
            pointer: 0,
            length: 0,
            capacity: 0,
            input_unchanged: true,
        })
        .is_err()
    );
}

#[test]
fn aggregate_target_declaration_budget_is_exact_and_plus_one() {
    let (_sources, linear, linux) = authorities();
    let seed = raw_v1(&linear, &linux);
    let mut exact = seed.clone();
    let mut row = seed.javascript[0].clone();
    row.parameters.clear();
    exact.javascript = vec![row.clone(); MAX_TARGET_FUNCTIONS];
    exact.webassembly.clear();
    exact.native_linux_x86_64.clear();
    assert!(super::input_within_limits(&exact));

    let mut plus = seed;
    plus.javascript = vec![row; MAX_TARGET_FUNCTIONS + 1];
    plus.webassembly.clear();
    plus.native_linux_x86_64.clear();
    assert_eq!(verify_v1(plus, &linear, &linux).expect_err("plus one")[0].code(), "ZRYNA-R3201");
}

#[test]
#[ignore = "proportional exact/+1 65,536-record boundary retained by full preflight"]
fn record_declaration_budget_is_exact_and_plus_one() {
    let (_sources, linear, linux) = authorities();
    let seed = raw_v1(&linear, &linux);
    let mut row = seed.records[0].clone();
    row.fields.clear();
    let mut exact = seed.clone();
    exact.records = vec![row.clone(); MAX_LAYOUT_REFERENCES];
    assert!(super::input_within_limits(&exact));
    let mut plus = seed;
    plus.records = vec![row; MAX_LAYOUT_REFERENCES + 1];
    assert_eq!(verify_v1(plus, &linear, &linux).expect_err("plus one")[0].code(), "ZRYNA-R3201");
}

#[test]
#[ignore = "proportional exact/+1 65,536-child boundary retained by full preflight"]
fn nested_declaration_budget_is_exact_and_plus_one() {
    let (_sources, linear, linux) = authorities();
    let seed = raw_v1(&linear, &linux);
    let mut exact = seed.clone();
    for operation in &mut exact.operations {
        operation.parameters.clear();
        operation.statuses.clear();
    }
    for helper in &mut exact.javascript {
        helper.parameters.clear();
    }
    for function in &mut exact.webassembly {
        function.parameters.clear();
        function.results.clear();
    }
    for function in &mut exact.native_linux_x86_64 {
        function.parameters.clear();
    }
    for record in &mut exact.records {
        record.fields.clear();
    }
    exact.statuses.clear();
    exact.layout_claims.clear();
    exact.operations[0].statuses = vec![0; MAX_DECLARATION_CHILDREN];
    assert!(super::input_within_limits(&exact));
    exact.operations[0].statuses.push(0);
    assert_eq!(verify_v1(exact, &linear, &linux).expect_err("plus one")[0].code(), "ZRYNA-R3201");
}

#[test]
#[ignore = "proportional exact/+1 16 MiB checked-header boundary retained by full preflight"]
fn checked_header_budget_is_exact_and_plus_one() {
    let (_sources, linear, linux) = authorities();
    let mut raw = raw_v1(&linear, &linux);
    raw.native_header = vec![b' '; MAX_CHECKED_HEADER_BYTES];
    assert!(super::input_within_limits(&raw));
    raw.native_header.push(b' ');
    assert_eq!(verify_v1(raw, &linear, &linux).expect_err("plus one")[0].code(), "ZRYNA-R3201");
}
