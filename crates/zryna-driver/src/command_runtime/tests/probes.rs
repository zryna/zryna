use std::time::Instant;

use wasm_encoder::{
    CanonicalOption, ComponentBuilder, ComponentExportKind, ComponentExportSection,
    ComponentInstanceSection, ComponentSection, ComponentValType, ExportKind, ModuleArg,
};

use super::{
    super::{CommandHostPolicy, prepare_command_self_check, session::Session},
    fixtures, probe_audit, probe_interfaces,
    probe_modules::{self, Action},
};

/// Test-only admission token for independently authored lower-boundary fixtures.
pub(crate) struct Probe {
    bytes: Vec<u8>,
}

impl Probe {
    pub(crate) fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

fn probe(filesystem: bool, action: Action) -> Probe {
    let bytes = probe_bytes(filesystem, action, &probe_modules::memory());
    probe_audit::audit(&bytes)
        .expect("test fixture satisfies its separate bounded lower-boundary envelope");
    Probe { bytes }
}

fn probe_bytes(filesystem: bool, action: Action, memory_module: &wasm_encoder::Module) -> Vec<u8> {
    let mut component = ComponentBuilder::default();
    let host = if filesystem {
        probe_interfaces::filesystem(&mut component)
    } else {
        probe_interfaces::environment(&mut component)
    };
    let memory = component.core_module(None, memory_module);
    let caller = component.core_module(None, &probe_modules::caller(action));
    let owned = component.core_instantiate(None, memory, []);
    let memory = component.core_alias_export(None, owned, "memory", ExportKind::Memory);
    let realloc = component.core_alias_export(None, owned, "realloc", ExportKind::Func);
    let lowered = component.lower_func(
        None,
        host,
        [CanonicalOption::UTF8, CanonicalOption::Memory(memory), CanonicalOption::Realloc(realloc)],
    );
    // One synthetic binding has no module allocation: two actual modules, three core index entries.
    let binding = component.core_instantiate_exports(None, [("deny", ExportKind::Func, lowered)]);
    let caller = component.core_instantiate(None, caller, [("host", ModuleArg::Instance(binding))]);
    let run = component.core_alias_export(None, caller, "run", ExportKind::Func);
    let (result_type, result) = component.type_defined(None);
    result.result(None, None);
    let (function_type, mut function) = component.type_function(None);
    function.params([] as [(&str, ComponentValType); 0]);
    function.result(Some(ComponentValType::Type(result_type)));
    let lifted = component.lift_func(None, run, function_type, []);
    let instance_index = component.instance_count();
    let mut bytes = component.finish();
    let mut instances = ComponentInstanceSection::new();
    instances.export_items([("run", ComponentExportKind::Func, lifted)]);
    instances.append_to_component(&mut bytes);
    let mut exports = ComponentExportSection::new();
    exports.export("wasi:cli/run@0.2.12", ComponentExportKind::Instance, instance_index, None);
    exports.append_to_component(&mut bytes);
    bytes
}

#[test]
fn actual_environment_and_filesystem_calls_are_denied_and_invalidate_their_stores() {
    for (filesystem, interface, operation) in [
        (false, "wasi:cli/environment@0.2.12", "get-environment"),
        (true, "wasi:filesystem/preopens@0.2.12", "get-directories"),
    ] {
        let probe = probe(filesystem, Action::DeniedCall);
        let mut session = Session::probe(&probe).expect("typed denied imports instantiate");
        assert!(session.invoke().is_err(), "denied calls cannot return data or resource handles");
        assert_eq!(session.denial_entries, 1);
        let denial = session.denial.as_ref().expect("actual host callback entry observed");
        assert_eq!(denial.interface.as_ref(), interface);
        assert_eq!(denial.operation.as_ref(), operation);
        assert!(!denial.resource_drop);
        assert!(
            session
                .invoke()
                .expect_err("fatal store cannot be reused")
                .message()
                .contains("invalidated")
        );
    }
    let sources = fixtures::sources();
    let policy = CommandHostPolicy::deny_all();
    let fresh = prepare_command_self_check(
        &fixtures::frontend(),
        &sources,
        &fixtures::wit(),
        "add",
        &[20, 22],
        42,
        policy,
    )
    .expect("fresh authentic production command");
    assert_eq!(fresh.execute(policy).expect("fresh store recovery"), Ok(()));
}

#[test]
fn infinite_loop_and_invalid_discriminant_are_fatal_without_host_entries() {
    for action in [Action::Loop, Action::InvalidResult] {
        let mut session =
            Session::probe(&probe(false, action)).expect("independent control component");
        let start = Instant::now();
        assert!(session.invoke().is_err());
        assert!(start.elapsed() <= super::super::envelope::EXECUTION_DEADLINE);
        assert_eq!(session.denial_entries, 0);
        assert!(session.denial.is_none());
        assert!(session.invoke().is_err());
    }
}

#[test]
fn probe_envelope_rejects_third_actual_instance_and_second_synthetic_binding() {
    let probe = probe(false, Action::DeniedCall);
    probe_audit::audit(probe.bytes()).expect("accepted probe topology");
    let mut actual = probe.bytes().to_vec();
    let mut extra = wasm_encoder::InstanceSection::new();
    extra.instantiate(0, [] as [(&str, ModuleArg); 0]);
    extra.append_to_component(&mut actual);
    assert!(probe_audit::audit(&actual).is_err());
    let mut synthetic = probe.bytes().to_vec();
    let mut extra = wasm_encoder::InstanceSection::new();
    extra.export_items([("deny", ExportKind::Func, 1)]);
    extra.append_to_component(&mut synthetic);
    assert!(probe_audit::audit(&synthetic).is_err());
}

#[test]
fn probe_memory_admits_one_page_and_rejects_first_extra_page() {
    let exact = probe_bytes(false, Action::DeniedCall, &probe_modules::memory_with_pages(1));
    probe_audit::audit(&exact).expect("one fixed 64KiB page");
    let extra = probe_bytes(false, Action::DeniedCall, &probe_modules::memory_with_pages(2));
    assert_eq!(
        probe_audit::audit(&extra).expect_err("first extra page"),
        "probe memory exceeds one fixed 64KiB page"
    );
}

#[test]
fn actual_denied_callback_rejects_a_linker_owned_by_another_store() {
    use super::super::{
        denied::{self, DeniedState},
        envelope,
    };
    let probe = probe(false, Action::DeniedCall);
    let engine = envelope::engine().expect("restricted engine");
    let component =
        wasmtime::component::Component::new(&engine, probe.bytes()).expect("admitted probe");
    let limits = || {
        wasmtime::StoreLimitsBuilder::new()
            .instances(2)
            .memories(1)
            .tables(0)
            .memory_size(64 * 1024)
            .table_elements(0)
            .build()
    };
    let original = DeniedState::new(limits()).expect("first owner");
    let linker = denied::linker(&engine, &component, &original).expect("first-owner callbacks");
    let mut store =
        wasmtime::Store::new(&engine, DeniedState::new(limits()).expect("different owner"));
    store.limiter(|state| &mut state.limits);
    store.set_fuel(envelope::FUEL).expect("bounded fixture fuel");
    store.set_epoch_deadline(1);
    let instance =
        linker.instantiate(&mut store, &component).expect("typed shape is valid in another store");
    let interface =
        instance.get_export_index(&mut store, None, "wasi:cli/run@0.2.12").expect("run interface");
    let run = instance.get_export_index(&mut store, Some(&interface), "run").expect("run function");
    let run =
        instance.get_typed_func::<(), (Result<(), ()>,)>(&mut store, &run).expect("typed run");
    assert!(run.call(&mut store, ()).is_err());
    assert!(store.data().fatal, "the actual callback observed the mismatched owner");
    assert_eq!(store.data().entries, 0, "foreign calls cannot claim another store's denial entry");
    assert!(store.data().first_denial.is_none());
}
