//! Exact typed invocation of the retained scalar ESM authority.

use std::path::Path;

use zryna_abi::{Invocation, RawHostScalar, ScalarOutcome, ScalarTarget, ScalarType};
use zryna_diagnostics::Diagnostic;

use super::{ScalarAdapterHost, VerifiedScalarEsm};
use crate::{pipeline_runtime::normalize_carrier, runtime::NodeRuntimeCapability};

pub(super) const CALL: &str = include_str!("scalar_call.js");

impl VerifiedScalarEsm {
    /// Revalidates the host-bound interface and prepares only its retained ABI export.
    pub(crate) fn invoke_node(
        &self,
        runtime: &NodeRuntimeCapability,
        working_directory: &Path,
        request: Invocation,
    ) -> Result<ScalarOutcome, Diagnostic> {
        self.require_host(ScalarAdapterHost::Node)?;
        let invocation = self.scalar_abi().prepare_invocation(request).map_err(|error| {
            Diagnostic::error(
                error.code(),
                None,
                "scalar consumer invocation does not match the retained interface",
                "use an exact export name, arity and scalar argument types",
            )
        })?;
        let export = invocation.export();
        let name = json(&export.javascript_name().as_str())?;
        let signature = json(&serde_json::json!([{
            "name": export.javascript_name().as_str(),
            "parameters": export.parameters(),
            "result": export.result(),
        }]))?;
        let arguments = crate::pipeline_runtime::render_javascript_arguments(&invocation);
        let digest = json(&crate::pipeline::hex_sha256(&self.artifact_sha256))?;
        let frame = match export.result() {
            ScalarType::I32 => {
                concat!("const frame = Buffer.alloc(4);\n", "frame.writeInt32LE(result.value);\n",)
            }
            ScalarType::Bool => "const frame = Uint8Array.of(result.value ? 1 : 0);\n",
        };
        let script = format!(
            "import {{ createHash }} from 'node:crypto';\n\
             let bytes = Buffer.alloc({artifact_bytes});\n\
             let offset = 0;\n\
             for await (const chunk of process.stdin) {{\n\
               if (offset + chunk.length > bytes.length) process.exit(70);\n\
               bytes.set(chunk, offset); offset += chunk.length;\n\
             }}\n\
             if (offset !== bytes.length) process.exit(70);\n\
             if (createHash('sha256').update(bytes).digest('hex') !== {digest}) process.exit(70);\n\
             {CALL}\n\
             let url = 'data:text/javascript;base64,' + bytes.toString('base64');\n\
             bytes = null;\n\
             const module = await import(url); url = null;\n\
             const result = scalarCall({signature}, module, {name}, [{arguments}]);\n\
             {frame}\
             process.stdout.write(frame);\n",
            artifact_bytes = self.artifact.source.len(),
        );
        // Revalidation follows request rendering and immediately precedes retained-byte transport.
        let source = self.javascript_source()?;
        let frame = runtime.run_inline_module(
            script.as_bytes(),
            source.as_bytes(),
            working_directory,
            match export.result() {
                ScalarType::Bool => 1,
                ScalarType::I32 => 4,
            },
        )?;
        let carrier = match (export.result(), frame.as_slice()) {
            (ScalarType::Bool, [0]) => RawHostScalar::JavaScriptBool(false),
            (ScalarType::Bool, [1]) => RawHostScalar::JavaScriptBool(true),
            (ScalarType::I32, [a, b, c, d]) => {
                RawHostScalar::JavaScriptNumber(f64::from(i32::from_le_bytes([*a, *b, *c, *d])))
            }
            _ => return Err(invalid("ZRYNA-R3006", "scalar consumer returned an invalid frame")),
        };
        Ok(normalize_carrier(ScalarTarget::JavaScript, export.result(), carrier))
    }

    pub(super) fn require_host(&self, host: ScalarAdapterHost) -> Result<(), Diagnostic> {
        if self.host != host {
            return Err(invalid("ZRYNA-D3870", "scalar consumer host differs from the sealed row"));
        }
        Ok(())
    }

    #[cfg(test)]
    pub(super) fn verify_host_view(
        &self,
        policy: super::raw::HostPolicy,
    ) -> Result<Self, Vec<Diagnostic>> {
        self.revalidate().map_err(|error| vec![error])?;
        let host = super::verify_host_policy(&policy)?;
        let interface_sha256 = super::interface_identity(host, self.scalar_abi());
        let binding_sha256 = super::binding_identity(
            &interface_sha256,
            &self.graph_sha256,
            &self.artifact_sha256,
            u64::try_from(self.artifact.source.len()).expect("bounded artifact length"),
        );
        Ok(Self {
            graph_sha256: self.graph_sha256,
            abi: self.abi.clone(),
            artifact: std::sync::Arc::clone(&self.artifact),
            host,
            artifact_sha256: self.artifact_sha256,
            interface_sha256,
            binding_sha256,
        })
    }
}

fn json(value: &impl serde::Serialize) -> Result<String, Diagnostic> {
    serde_json::to_string(value)
        .map_err(|_| invalid("ZRYNA-D3871", "scalar consumer request could not be encoded"))
}

fn invalid(code: &'static str, message: &'static str) -> Diagnostic {
    Diagnostic::error(code, None, message, "discard the request and repeat sealed preparation")
}
