//! An explicitly exported derivative of the frozen Struct source executes after a real v4 seal.
use zryna_ir::generic_v1::owned_v2 as owned;
use zryna_layout::StorageTarget;
use zryna_semantics::bounded_generics_v1::{
    SemanticInput,
    body_types::check_body_types,
    instantiation::{
        layouts::verify_layouts,
        owned_v4::{discover, produce_claim},
    },
    resolve_declarations,
};
use zryna_source::{SourceFileInput, SourceMap};
use zryna_syntax::v5::{decode_snapshot, verify_snapshot};

#[test]
fn frozen_struct_construction_copy_observation_executes_without_runtime_allocation() {
    let original = include_str!("../../../../tests/m7-generic-struct-v4/construction.zry");
    let source = include_str!(
        "../../../zryna-driver/tests/generic_struct_v4/fixtures/construction-exported.zry"
    );
    assert_eq!(source, format!("{original}\nexport function entry(): i32 {{ return score(); }}\n"));
    let sources =
        SourceMap::build(vec![SourceFileInput { path: "main.zry".into(), text: source.into() }])
            .expect("source");
    let syntax = verify_snapshot(
        decode_snapshot(include_bytes!(
            "../../../zryna-driver/tests/generic_struct_v4/fixtures/construction-exported.json"
        ))
        .expect("packet"),
        &sources,
    )
    .expect("authentic source");
    let entry = sources.verify_file_id(0).expect("entry");
    let declarations =
        resolve_declarations(SemanticInput::try_new(&syntax, &sources, entry).expect("input"))
            .expect("declarations");
    let bodies = check_body_types(&declarations).expect("body types");
    let instances = discover(&bodies).expect("v4 originals and discovery");
    let linear = verify_layouts(instances.instances(), StorageTarget::Linear32V1).expect("linear");
    let linux = verify_layouts(instances.instances(), StorageTarget::LinuxX8664V1).expect("linux");
    let runtime = zryna_ownership_runtime_abi::generic_v1::verify_v1(
        zryna_ownership_runtime_abi::generic_v1::raw_v1(&linear, &linux).expect("runtime claims"),
        &linear,
        &linux,
    )
    .expect("runtime");
    let claim = produce_claim(&instances, &linear, &linux).expect("claims");
    let wire = owned::wire::v4::encode(&claim).expect("wire");
    let program = owned::verify_v4(
        owned::wire::v4::decode(&wire).expect("decode"),
        &syntax,
        &sources,
        entry,
        &linear,
        &linux,
        &runtime,
    )
    .expect("v4 authority");
    let artifact = super::emit(&program).expect("emit");
    assert_eq!(artifact, super::emit(&program).expect("deterministic"));
    let name = program
        .scalar_abi()
        .exports()
        .next()
        .expect("scalar export")
        .webassembly_name()
        .as_str()
        .to_owned();
    let script = format!(
        r"
        let calls = 0;
        const rt = {{ memory: new WebAssembly.Memory({{ initial: 1024, maximum: 1024 }}),
          stringFromUtf8Copy() {{ calls++; return 1; }},
          stringClone() {{ calls++; return 1; }}, stringRelease() {{ calls++; return 1; }} }};
        const bytes = new Uint8Array({:?});
        const module = await WebAssembly.instantiate(bytes, {{ {:?}: rt }});
        if (module.instance.exports[{:?}]() !== 7 || calls !== 0)
          throw Error('wrong frozen scalar outcome or unexpected runtime call');
    ",
        artifact.bytes(),
        super::runtime::NAMESPACE,
        name
    );
    let output = std::process::Command::new("node")
        .args(["--input-type=module", "-e", &script])
        .output()
        .expect("Node");
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
}
