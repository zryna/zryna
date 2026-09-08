use zryna_abi::{Invocation, ScalarOutcome, ScalarValue};

use super::*;
use crate::runtime::NodeRuntimeCapability;

const CORPUS: &str = include_str!("../../../../../tests/scalar-host/corpus.js");

fn hex(bytes: &[u8; 32]) -> String {
    crate::pipeline::hex_sha256(bytes)
}

fn host_view(interface: &VerifiedScalarEsm, host: ScalarAdapterHost) -> VerifiedScalarEsm {
    interface
        .verify_host_view(&raw::HostPolicy {
            boundary: host.identity().to_owned(),
            required_interfaces: Vec::new(),
        })
        .expect("explicit pure host view")
}

fn request(interface: &VerifiedScalarEsm, host: ScalarAdapterHost) -> serde_json::Value {
    interface.require_host(host).expect("retained host binding");
    let exports = interface
        .scalar_abi()
        .exports()
        .map(|export| {
            serde_json::json!({
                "name": export.javascript_name().as_str(),
                "parameters": export.parameters(), "result": export.result(),
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "host": host.identity(), "requiredInterfaces": [],
        "source": interface.javascript_source().expect("immediate retained byte revalidation"),
        "exports": exports,
        "artifactSha256": hex(&interface.artifact_sha256),
        "interfaceSha256": hex(&interface.interface_sha256),
        "bindingSha256": hex(&interface.binding_sha256),
        "graphSha256": hex(&interface.graph_sha256),
        "call": consumer::CALL, "corpus": CORPUS,
    })
}

fn node_corpus(
    runtime: &NodeRuntimeCapability,
    interface: &VerifiedScalarEsm,
) -> serde_json::Value {
    let packet = serde_json::to_vec(&request(interface, ScalarAdapterHost::Node)).expect("packet");
    let script = concat!(
        "const chunks = []; for await (const chunk of process.stdin) chunks.push(chunk);\n",
        "const input = JSON.parse(Buffer.concat(chunks));\n",
        "verifyFixtureBinding(input, 'js-node');\n",
        "const call = (0, eval)(input.call + ';scalarCall');\n",
        "const corpus = (0, eval)(input.corpus + ';scalarCorpus');\n",
        "const module = await import('data:text/javascript;base64,' + Buffer.from(input.source).toString('base64'));\n",
        "process.stdout.write(JSON.stringify({...corpus(module,input.exports,call),\n",
        "artifactSha256: input.artifactSha256, interfaceSha256: input.interfaceSha256, bindingSha256: input.bindingSha256}));\n",
    );
    let binding = repository_root().join("tests/scalar-host/binding.mjs");
    let binding = serde_json::to_string(&crate::runtime::node_compatible_path(&binding))
        .expect("binding path");
    let script = format!(
        "import {{ pathToFileURL }} from 'node:url';\n\
         const {{ verifyFixtureBinding }} = await import(pathToFileURL({binding}));\n{script}"
    );
    let bytes = runtime
        .run_inline_module(script.as_bytes(), &packet, &repository_root(), 64 * 1024)
        .expect("real pinned Node carrier corpus");
    serde_json::from_slice(&bytes).expect("bounded typed conformance report")
}

fn assert_corpus(report: &serde_json::Value) {
    assert_eq!(report["positiveCount"], 14);
    assert_eq!(report["negativeCount"], 39);
    assert_eq!(report["sentinelEntries"], 0);
    assert_eq!(report["requiredInterfaces"], serde_json::json!([]));
    let returned = report["returned"].as_array().expect("typed returns");
    assert_eq!(returned.len(), 14);
    assert_eq!(&returned[..7], &returned[7..]);
    assert_eq!(returned[0], serde_json::json!({"type": "i32", "value": 42}));
    assert_eq!(returned[1], serde_json::json!({"type": "i32", "value": -2_147_483_648_i32}));
}

#[test]
fn sealed_node_consumer_returns_exact_types_and_runs_hostile_carriers() {
    let fixture = Fixture::new("node-consumer", SCALAR_SOURCE);
    let interface =
        fixture.verify(fixture.claim(ScalarAdapterHost::Node)).expect("sealed interface");
    let runtime =
        NodeRuntimeCapability::discover(&node_executable(), &repository_root()).expect("Node");
    for (name, values, expected) in [
        ("add", vec![ScalarValue::I32(20), ScalarValue::I32(22)], ScalarValue::I32(42)),
        ("add", vec![ScalarValue::I32(i32::MAX), ScalarValue::I32(1)], ScalarValue::I32(i32::MIN)),
        ("identity", vec![ScalarValue::Bool(false)], ScalarValue::Bool(false)),
        ("identity", vec![ScalarValue::Bool(true)], ScalarValue::Bool(true)),
    ] {
        for _ in 0..2 {
            assert_eq!(
                interface
                    .invoke_node(
                        &runtime,
                        &repository_root(),
                        Invocation::new(name.to_owned(), values.clone())
                    )
                    .expect("typed sealed call"),
                ScalarOutcome::Returned { value: expected }
            );
        }
    }
    assert_corpus(&node_corpus(&runtime, &interface));
}

#[test]
fn rejects_stale_host_artifact_and_typed_requests_before_runtime_then_recovers() {
    let fixture = Fixture::new("consumer-rejection", SCALAR_SOURCE);
    let interface =
        fixture.verify(fixture.claim(ScalarAdapterHost::Node)).expect("sealed interface");
    let runtime =
        NodeRuntimeCapability::discover(&node_executable(), &repository_root()).expect("Node");
    // A nonexistent working directory makes accidental process entry observable as a runtime error.
    let absent = fixture.workspace.path.join("absent");
    for (request, code) in [
        (Invocation::new("missing".to_owned(), vec![]), "ZRYNA-B2101"),
        (Invocation::new("add".to_owned(), vec![ScalarValue::I32(20)]), "ZRYNA-B2102"),
        (Invocation::new("identity".to_owned(), vec![ScalarValue::I32(1)]), "ZRYNA-B2103"),
    ] {
        assert_eq!(
            interface.invoke_node(&runtime, &absent, request).expect_err("before process").code(),
            code
        );
    }
    let valid =
        || Invocation::new("add".to_owned(), vec![ScalarValue::I32(20), ScalarValue::I32(22)]);
    let browser = host_view(&interface, ScalarAdapterHost::Browser);
    assert_eq!(
        browser.invoke_node(&runtime, &absent, valid()).expect_err("host mismatch").code(),
        "ZRYNA-D3870"
    );
    assert!(std::sync::Arc::ptr_eq(&interface.artifact, &browser.artifact));
    assert_eq!(interface.artifact_sha256, browser.artifact_sha256);
    assert_ne!(interface.interface_sha256, browser.interface_sha256);
    assert_ne!(interface.binding_sha256, browser.binding_sha256);
    assert_eq!(
        interface
            .verify_host_view(&raw::HostPolicy {
                boundary: "js-browser".to_owned(),
                required_interfaces: vec!["network".to_owned()],
            })
            .expect_err("nonempty host interface request")[0]
            .code(),
        "ZRYNA-D3815"
    );
    for field in 0..4 {
        let mut stale = host_view(&interface, ScalarAdapterHost::Node);
        match field {
            0 => stale.binding_sha256[0] ^= 1,
            1 => stale.interface_sha256[0] ^= 1,
            2 => stale.graph_sha256[0] ^= 1,
            _ => {
                std::sync::Arc::make_mut(&mut stale.artifact).source.push_str("\n// substituted\n");
            }
        }
        assert_eq!(
            stale.invoke_node(&runtime, &absent, valid()).expect_err("stale seal").code(),
            "ZRYNA-D3810"
        );
    }
    assert_eq!(
        interface.invoke_node(&runtime, &repository_root(), valid()).expect("recovery"),
        ScalarOutcome::Returned { value: ScalarValue::I32(42) }
    );
}

#[test]
fn maximum_typed_scalar_invocation_fits_existing_inline_script_budget() {
    let parameters = (0..256).map(|index| format!("p{index}: i32")).collect::<Vec<_>>().join(", ");
    let source = format!("export function exact({parameters}): i32 {{ return p0; }}\n");
    let fixture = Fixture::new("maximum-consumer-arity", &source);
    let interface =
        fixture.verify(fixture.claim(ScalarAdapterHost::Node)).expect("sealed interface");
    let runtime =
        NodeRuntimeCapability::discover(&node_executable(), &repository_root()).expect("Node");
    let request = Invocation::new("exact".to_owned(), vec![ScalarValue::I32(i32::MIN); 256]);
    assert_eq!(
        interface
            .invoke_node(&runtime, &repository_root(), request)
            .expect("maximum typed request"),
        ScalarOutcome::Returned { value: ScalarValue::I32(i32::MIN) }
    );
}

#[test]
#[ignore = "requires separately scheduled full 32 MiB transport resource proof"]
fn retained_esm_transport_accepts_exact_ceiling_and_rejects_first_extra() {
    let runtime =
        NodeRuntimeCapability::discover(&node_executable(), &repository_root()).expect("Node");
    // Independently constructed transport input, not compiler/interface conformance authority.
    let mut bytes = b"export function add(a,b) { return (a+b)|0; }\n".to_vec();
    bytes.resize(32 * 1024 * 1024, b' ');
    let script = concat!(
        "let bytes=Buffer.alloc(33554432); let offset=0;\n",
        "for await(const chunk of process.stdin) { bytes.set(chunk,offset); offset+=chunk.length; }\n",
        "if(offset!==33554432) process.exit(70);\n",
        "let url='data:text/javascript;base64,'+bytes.toString('base64'); bytes=null;\n",
        "const module=await import(url); url=null;\n",
        "const frame=Buffer.alloc(4); frame.writeInt32LE(module.add(20,22)); process.stdout.write(frame);\n",
    );
    assert_eq!(
        runtime
            .run_inline_module(script.as_bytes(), &bytes, &repository_root(), 4)
            .expect("exact retained artifact ceiling imports under existing production deadline"),
        42_i32.to_le_bytes()
    );
    bytes.push(b' ');
    assert_eq!(
        runtime
            .run_inline_module(script.as_bytes(), &bytes, &repository_root().join("absent"), 4)
            .expect_err("first extra byte rejects before process launch")
            .code(),
        "ZRYNA-R3004"
    );
}

#[test]
#[ignore = "requires separately acquired and reviewed pinned real-browser fixture"]
fn pinned_real_browser_and_node_execute_the_same_sealed_scalar_corpus() {
    let fixture = Fixture::new("both-hosts", SCALAR_SOURCE);
    let node = fixture.verify(fixture.claim(ScalarAdapterHost::Node)).expect("Node seal");
    let browser = host_view(&node, ScalarAdapterHost::Browser);
    let runtime =
        NodeRuntimeCapability::discover(&node_executable(), &repository_root()).expect("Node");
    let node_report = node_corpus(&runtime, &node);
    assert_corpus(&node_report);
    let mut packet = request(&browser, ScalarAdapterHost::Browser);
    packet["browserRoot"] = serde_json::Value::String(
        env::var("ZRYNA_TEST_BROWSER_ROOT")
            .expect("explicit reviewed private browser fixture directory"),
    );
    packet["runtimeDirectory"] = serde_json::to_value(crate::runtime::node_compatible_path(
        &fixture.workspace.path.join("browser-runtime"),
    ))
    .expect("private browser runtime path");
    let runner = repository_root().join("tests/scalar-host/browser-fixture.mjs");
    let runner =
        serde_json::to_string(&crate::runtime::node_compatible_path(&runner)).expect("runner path");
    let script = format!(
        "import {{ pathToFileURL }} from 'node:url';\n\
         const {{ runBrowserFixture }} = await import(pathToFileURL({runner}));\n\
         const chunks=[]; for await(const chunk of process.stdin) chunks.push(chunk);\n\
         const report=await runBrowserFixture(JSON.parse(Buffer.concat(chunks)));\n\
         process.stdout.write(JSON.stringify(report));\n"
    );
    let bytes = runtime
        .run_browser_fixture(
            script.as_bytes(),
            &serde_json::to_vec(&packet).expect("packet"),
            &repository_root(),
        )
        .expect("real pinned browser execution and descendant cleanup");
    let report: serde_json::Value = serde_json::from_slice(&bytes).expect("typed browser report");
    assert_corpus(&report);
    for field in ["returned", "rejected", "artifactSha256", "requiredInterfaces"] {
        assert_eq!(report[field], node_report[field], "cross-host {field}");
    }
    assert_eq!(report["interfaceSha256"], hex(&browser.interface_sha256));
    assert_eq!(report["bindingSha256"], hex(&browser.binding_sha256));
    assert_ne!(report["interfaceSha256"], node_report["interfaceSha256"]);
    assert_ne!(report["bindingSha256"], node_report["bindingSha256"]);
    assert_eq!(report["browserVersion"], "153.0.8010.12");
    assert_eq!(report["runnerVersion"], "1.63.0");
    assert_eq!(report["requests"], 0);
    assert_eq!(report["workers"], 0);
    assert_eq!(report["closed"], true);
}
