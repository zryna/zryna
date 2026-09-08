use std::collections::{BTreeMap, BTreeSet};

use sha2::{Digest, Sha256};
use zryna_backend_webassembly::WitWorldAudit;
use zryna_diagnostics::Diagnostic;
use zryna_source::SourceMap;

use super::{INVALID, error, model::Language};

#[derive(Clone, Debug)]
pub(super) enum VerifiedLanguage {
    I32V1 {
        program: zryna_ir::VerifiedProgram,
        sources: SourceMap,
    },
    ControlFlowV1 {
        program: zryna_ir::control_flow_v1::VerifiedProgram,
        sources: SourceMap,
    },
    DataOwnershipV1 {
        program: zryna_semantics::data_ownership_v1::VerifiedProgram,
        sources: SourceMap,
    },
}

impl VerifiedLanguage {
    fn language(&self) -> Language {
        match self {
            Self::I32V1 { .. } => Language::I32V1,
            Self::ControlFlowV1 { .. } => Language::ControlFlowV1,
            Self::DataOwnershipV1 { .. } => Language::DataOwnershipV1,
        }
    }

    fn source_fingerprint(&self) -> Result<[u8; 32], Vec<Diagnostic>> {
        let valid = match self {
            Self::I32V1 { program, sources } => {
                let mut spans = program.functions().flat_map(|function| function.expressions());
                let first = spans.next();
                first.is_some()
                    && first
                        .into_iter()
                        .chain(spans)
                        .all(|expression| sources.resolve(expression.span).is_ok())
            }
            Self::ControlFlowV1 { program, sources } => {
                let mut modules = program.modules();
                let first = modules.next();
                first.is_some()
                    && first
                        .into_iter()
                        .chain(modules)
                        .all(|module| sources.source(module.source_file()).is_some())
            }
            Self::DataOwnershipV1 { program, sources } => {
                program.verified_ir().source_map_identity() == sources.identity()
            }
        };
        if !valid {
            return Err(vec![error(
                INVALID,
                "verified language program is not bound to its exact source authority",
            )]);
        }
        let sources = match self {
            Self::I32V1 { sources, .. }
            | Self::ControlFlowV1 { sources, .. }
            | Self::DataOwnershipV1 { sources, .. } => sources,
        };
        let mut bytes = Vec::new();
        for index in 0..sources.len() {
            let id = sources
                .verify_file_id(u32::try_from(index).expect("bounded source map"))
                .map_err(|_| vec![error(INVALID, "cannot bind verified source authority")])?;
            let source = sources.source(id).expect("verified dense source id");
            bytes.extend_from_slice(source.path().as_bytes());
            bytes.push(0);
            bytes.extend_from_slice(source.text().as_bytes());
            bytes.push(0xff);
        }
        Ok(Sha256::digest(bytes).into())
    }

    fn fingerprint(&self) -> [u8; 32] {
        let bytes = match self {
            Self::I32V1 { program, .. } => {
                let mut bytes = Vec::new();
                for function in program.functions() {
                    bytes.extend_from_slice(function.export_name().as_str().as_bytes());
                    bytes.push(0);
                    bytes.extend_from_slice(format!("{:?}", function.parameters()).as_bytes());
                    bytes.extend_from_slice(format!("{:?}", function.return_type()).as_bytes());
                    bytes.extend_from_slice(
                        &serde_json::to_vec(function.expressions())
                            .expect("verified scalar expressions serialize"),
                    );
                    bytes.extend_from_slice(format!("{:?}", function.body()).as_bytes());
                }
                bytes
            }
            // These opaque programs retain exact source-map identity across clones.
            Self::ControlFlowV1 { program, .. } => format!("{program:?}").into_bytes(),
            Self::DataOwnershipV1 { program, .. } => format!("{program:?}").into_bytes(),
        };
        Sha256::digest(bytes).into()
    }
}

#[derive(Clone, Debug)]
pub(super) struct InstanceAuthority {
    pub programs: Vec<VerifiedLanguage>,
}

#[derive(Clone, Debug)]
pub(super) struct Authorities {
    pub instances: BTreeMap<String, InstanceAuthority>,
    pub wit: Option<WitWorldAudit>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Binding {
    instances: Vec<(String, Vec<(Language, [u8; 32], [u8; 32])>)>,
    wit: Option<WitWorldAudit>,
}

impl Authorities {
    pub(super) fn binding(&self, expected: &BTreeSet<String>) -> Result<Binding, Vec<Diagnostic>> {
        if self.instances.keys().ne(expected.iter()) {
            return Err(vec![error(
                INVALID,
                "verified instance authorities do not match the fixed graph",
            )]);
        }
        let mut instances = Vec::with_capacity(self.instances.len());
        for (id, authority) in &self.instances {
            if authority.programs.len() > 3 {
                return Err(vec![error(
                    INVALID,
                    "instance sealed program authority bound (1..=3) exceeded",
                )]);
            }
            let mut languages = Vec::with_capacity(authority.programs.len());
            for program in &authority.programs {
                languages.push((
                    program.language(),
                    program.source_fingerprint()?,
                    program.fingerprint(),
                ));
            }
            languages.sort_by_key(|(language, _, _)| *language);
            if languages.is_empty() || languages.windows(2).any(|pair| pair[0].0 == pair[1].0) {
                return Err(vec![error(
                    INVALID,
                    "instance requires one distinct sealed authority per language",
                )]);
            }
            instances.push((id.clone(), languages));
        }
        Ok(Binding { instances, wit: self.wit.clone() })
    }
}

impl Binding {
    pub(super) fn has_language(&self, id: &str, language: Language) -> bool {
        self.instances
            .iter()
            .find(|(candidate, _)| candidate == id)
            .is_some_and(|(_, programs)| programs.iter().any(|entry| entry.0 == language))
    }

    pub(super) const fn wit(&self) -> Option<&WitWorldAudit> {
        self.wit.as_ref()
    }
}
