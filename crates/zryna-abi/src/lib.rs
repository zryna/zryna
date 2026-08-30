//! Verified, versioned scalar ABI contracts shared by every Zryna target.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Scalar ABI version implemented by this crate.
pub const SCALAR_ABI_V1: ScalarAbiVersion = ScalarAbiVersion::V1;
/// Maximum logical export-name bytes admitted by scalar ABI v1.
pub const MAX_LOGICAL_EXPORT_NAME_BYTES: usize = 128;
/// Maximum exports admitted by one scalar ABI module.
pub const MAX_ABI_EXPORTS: usize = 16_384;
/// Maximum parameters admitted by one scalar ABI export.
pub const MAX_ABI_PARAMETERS_PER_EXPORT: usize = 256;
/// Maximum parameters admitted across one scalar ABI module.
pub const MAX_ABI_PARAMETERS_PER_MODULE: usize = 262_144;
/// Maximum retained ABI violations, including the terminal budget violation.
pub const MAX_ABI_VIOLATIONS: usize = 256;

/// Stable scalar ABI versions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum ScalarAbiVersion {
    /// First strict scalar ABI.
    V1 = 1,
}

/// Scalar types exposed by ABI v1.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ScalarType {
    /// Canonical two-state Boolean.
    Bool,
    /// Signed 32-bit integer.
    I32,
}

/// A fully typed scalar value used for invocation and differential observation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", content = "value", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ScalarValue {
    /// Boolean value.
    Bool(bool),
    /// Signed 32-bit integer value.
    I32(i32),
}

impl ScalarValue {
    /// Returns the exact scalar type carried by this value.
    #[must_use]
    pub const fn ty(self) -> ScalarType {
        match self {
            Self::Bool(_) => ScalarType::Bool,
            Self::I32(_) => ScalarType::I32,
        }
    }
}

/// Target profiles defined by scalar ABI v1.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScalarTarget {
    /// ECMAScript module called through a strict JavaScript host wrapper.
    JavaScript,
    /// Direct core WebAssembly module.
    CoreWebAssembly,
    /// Linux x86-64 native artifact using the System V ABI.
    NativeLinuxX8664,
}

/// Raw scalar values observed at one target boundary.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RawHostScalar {
    /// JavaScript Boolean.
    JavaScriptBool(bool),
    /// JavaScript Number.
    JavaScriptNumber(f64),
    /// WebAssembly or native `i32` carrier.
    I32(i32),
}

/// Stable scalar host-boundary validation errors.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScalarBoundaryError {
    /// The target supplied the wrong carrier kind.
    TargetCarrierMismatch,
    /// A JavaScript Number was not one canonical signed 32-bit integer.
    InvalidJavaScriptI32(f64),
    /// A WebAssembly or native Boolean carrier was neither zero nor one.
    InvalidBoolCarrier(i32),
}

impl ScalarBoundaryError {
    /// Returns the stable machine-readable error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::TargetCarrierMismatch => "ZRYNA-B2001",
            Self::InvalidJavaScriptI32(_) => "ZRYNA-B2002",
            Self::InvalidBoolCarrier(_) => "ZRYNA-B2003",
        }
    }
}

/// Encodes one typed invocation argument for a target host boundary.
#[must_use]
pub const fn encode_argument(target: ScalarTarget, value: ScalarValue) -> RawHostScalar {
    match (target, value) {
        (ScalarTarget::JavaScript, ScalarValue::Bool(value)) => {
            RawHostScalar::JavaScriptBool(value)
        }
        (ScalarTarget::JavaScript, ScalarValue::I32(value)) => {
            RawHostScalar::JavaScriptNumber(value as f64)
        }
        (
            ScalarTarget::CoreWebAssembly | ScalarTarget::NativeLinuxX8664,
            ScalarValue::Bool(value),
        ) => RawHostScalar::I32(if value { 1 } else { 0 }),
        (
            ScalarTarget::CoreWebAssembly | ScalarTarget::NativeLinuxX8664,
            ScalarValue::I32(value),
        ) => RawHostScalar::I32(value),
    }
}

/// Normalizes one raw target result into a fully typed scalar value.
///
/// # Errors
///
/// Rejects a mismatched carrier, non-canonical JavaScript `i32`, or non-canonical Boolean lane.
pub fn normalize_result(
    target: ScalarTarget,
    expected: ScalarType,
    raw: RawHostScalar,
) -> Result<ScalarValue, ScalarBoundaryError> {
    decode_boundary_value(target, expected, raw)
}

/// Validates and decodes one raw target argument before user-code execution.
///
/// # Errors
///
/// Rejects a mismatched carrier, non-canonical JavaScript `i32`, or non-canonical Boolean lane.
pub fn decode_argument(
    target: ScalarTarget,
    expected: ScalarType,
    raw: RawHostScalar,
) -> Result<ScalarValue, ScalarBoundaryError> {
    decode_boundary_value(target, expected, raw)
}

fn decode_boundary_value(
    target: ScalarTarget,
    expected: ScalarType,
    raw: RawHostScalar,
) -> Result<ScalarValue, ScalarBoundaryError> {
    match (target, expected, raw) {
        (ScalarTarget::JavaScript, ScalarType::Bool, RawHostScalar::JavaScriptBool(value)) => {
            Ok(ScalarValue::Bool(value))
        }
        (ScalarTarget::JavaScript, ScalarType::I32, RawHostScalar::JavaScriptNumber(value)) => {
            if !value.is_finite()
                || value.fract() != 0.0
                || value < f64::from(i32::MIN)
                || value > f64::from(i32::MAX)
                || (value == 0.0 && value.is_sign_negative())
            {
                return Err(ScalarBoundaryError::InvalidJavaScriptI32(value));
            }
            // The finite, integral, signed-range checks above prove this narrowing exactly.
            #[allow(clippy::cast_possible_truncation)]
            let value = value as i32;
            Ok(ScalarValue::I32(value))
        }
        (
            ScalarTarget::CoreWebAssembly | ScalarTarget::NativeLinuxX8664,
            ScalarType::I32,
            RawHostScalar::I32(value),
        ) => Ok(ScalarValue::I32(value)),
        (
            ScalarTarget::CoreWebAssembly | ScalarTarget::NativeLinuxX8664,
            ScalarType::Bool,
            RawHostScalar::I32(0),
        ) => Ok(ScalarValue::Bool(false)),
        (
            ScalarTarget::CoreWebAssembly | ScalarTarget::NativeLinuxX8664,
            ScalarType::Bool,
            RawHostScalar::I32(1),
        ) => Ok(ScalarValue::Bool(true)),
        (
            ScalarTarget::CoreWebAssembly | ScalarTarget::NativeLinuxX8664,
            ScalarType::Bool,
            RawHostScalar::I32(value),
        ) => Err(ScalarBoundaryError::InvalidBoolCarrier(value)),
        _ => Err(ScalarBoundaryError::TargetCarrierMismatch),
    }
}

/// Stable trap categories compared by differential conformance.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ScalarTrapCode {
    /// Explicit language-level unreachable execution.
    Unreachable,
    /// Target engine reported an execution trap.
    TargetTrap,
}

/// Stable host-side failure categories that are not program traps.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ScalarHostErrorCode {
    /// Export lookup failed.
    UnknownExport,
    /// Invocation arguments failed ABI validation.
    InvalidInvocation,
    /// Target returned an invalid ABI representation.
    InvalidTargetResult,
    /// Target artifact could not be loaded or invoked.
    TargetUnavailable,
}

/// Typed target outcome. Process exit status is not a scalar result.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ScalarOutcome {
    /// Function returned one typed scalar.
    Returned {
        /// Returned value.
        value: ScalarValue,
    },
    /// Function trapped with one stable category.
    Trapped {
        /// Stable trap category.
        code: ScalarTrapCode,
    },
    /// Host validation, loading, or normalization failed outside program execution.
    HostError {
        /// Stable host-side failure category.
        code: ScalarHostErrorCode,
    },
}

/// Untrusted scalar ABI declarations.
pub mod raw {
    /// Value types claimed before scalar ABI verification.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub enum Type {
        /// No-result or unit value, unsupported by scalar ABI v1 exports.
        Unit,
        /// Boolean value.
        Bool,
        /// Signed 32-bit integer.
        I32,
    }

    /// Untrusted scalar signature.
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct Signature {
        pub(super) parameters: Vec<Type>,
        pub(super) result: Type,
    }

    impl Signature {
        /// Creates an untrusted signature claim.
        #[must_use]
        pub const fn new(parameters: Vec<Type>, result: Type) -> Self {
            Self { parameters, result }
        }
    }

    /// Untrusted exported scalar function claim.
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct Export {
        pub(super) logical_name: String,
        pub(super) signature: Signature,
    }

    impl Export {
        /// Creates an untrusted export claim.
        #[must_use]
        pub const fn new(logical_name: String, signature: Signature) -> Self {
            Self { logical_name, signature }
        }
    }

    /// Untrusted scalar ABI module claim.
    #[derive(Clone, Debug, Default, Eq, PartialEq)]
    pub struct Module {
        pub(super) exports: Vec<Export>,
    }

    impl Module {
        /// Creates an untrusted module claim.
        #[must_use]
        pub const fn new(exports: Vec<Export>) -> Self {
            Self { exports }
        }
    }
}

/// Stable scalar ABI violation kinds.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AbiViolationKind {
    /// Module contains too many exports.
    TooManyExports,
    /// One export contains too many parameters.
    TooManyParameters,
    /// Module contains too many parameters in aggregate.
    TooManyParametersInModule,
    /// Logical export spelling is invalid or reserved.
    InvalidLogicalName,
    /// Logical export is an exact duplicate.
    DuplicateLogicalName {
        /// Earlier declaration with the same exact name.
        first_index: usize,
    },
    /// Logical exports collide under the portable ASCII identity.
    PortableNameCollision {
        /// Earlier declaration with the same portable identity.
        first_index: usize,
    },
    /// Signature contains a value outside scalar ABI v1.
    UnsupportedScalarType,
    /// Additional violations were omitted at the deterministic diagnostic budget.
    ViolationBudgetExceeded,
}

/// One deterministic scalar ABI violation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AbiViolation {
    export_index: Option<usize>,
    kind: AbiViolationKind,
}

impl AbiViolation {
    /// Returns the affected export index, when the violation is export-local.
    #[must_use]
    pub const fn export_index(self) -> Option<usize> {
        self.export_index
    }

    /// Returns the exact violation kind.
    #[must_use]
    pub const fn kind(self) -> AbiViolationKind {
        self.kind
    }

    /// Returns the stable machine-readable violation code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self.kind {
            AbiViolationKind::InvalidLogicalName => "ZRYNA-B1001",
            AbiViolationKind::DuplicateLogicalName { .. } => "ZRYNA-B1002",
            AbiViolationKind::PortableNameCollision { .. } => "ZRYNA-B1003",
            AbiViolationKind::UnsupportedScalarType => "ZRYNA-B1004",
            AbiViolationKind::TooManyExports
            | AbiViolationKind::TooManyParameters
            | AbiViolationKind::TooManyParametersInModule => "ZRYNA-B1201",
            AbiViolationKind::ViolationBudgetExceeded => "ZRYNA-B1202",
        }
    }
}

#[derive(Default)]
struct ViolationBuffer {
    values: Vec<AbiViolation>,
}

impl ViolationBuffer {
    fn push(&mut self, violation: AbiViolation) {
        if self.values.len() < MAX_ABI_VIOLATIONS - 1 {
            self.values.push(violation);
        } else if self.values.len() == MAX_ABI_VIOLATIONS - 1 {
            self.values.push(AbiViolation {
                export_index: None,
                kind: AbiViolationKind::ViolationBudgetExceeded,
            });
        }
    }

    fn exhausted(&self) -> bool {
        self.values.len() == MAX_ABI_VIOLATIONS
    }

    fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

/// Verified logical export name.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct LogicalExportName(Box<str>);

impl LogicalExportName {
    /// Returns the exact logical spelling.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Verified JavaScript public export name.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct JavaScriptExportName(Box<str>);

impl JavaScriptExportName {
    /// Returns the exact ECMAScript module export spelling.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Verified core WebAssembly export name.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct WebAssemblyExportName(Box<str>);

impl WebAssemblyExportName {
    /// Returns the exact core WebAssembly export spelling.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Verified Linux x86-64 native symbol.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct NativeLinuxX8664Symbol(Box<str>);

impl NativeLinuxX8664Symbol {
    /// Returns the exact ELF symbol spelling.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct VerifiedExport {
    logical_name: LogicalExportName,
    javascript_name: JavaScriptExportName,
    webassembly_name: WebAssemblyExportName,
    native_linux_x86_64_symbol: NativeLinuxX8664Symbol,
    parameters: Vec<ScalarType>,
    result: ScalarType,
}

/// Module proven to satisfy scalar ABI v1.
///
/// Its fields and all verified target-name constructors are private. Only [`verify_v1`] can create
/// this authority object.
///
/// ```compile_fail
/// let _ = zryna_abi::VerifiedScalarAbiModule {
///     exports: Vec::new(),
///     lookup: std::collections::BTreeMap::new(),
/// };
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedScalarAbiModule {
    exports: Vec<VerifiedExport>,
    lookup: BTreeMap<Box<str>, usize>,
}

impl VerifiedScalarAbiModule {
    /// Returns the verified ABI version.
    #[must_use]
    pub const fn version(&self) -> ScalarAbiVersion {
        SCALAR_ABI_V1
    }

    /// Iterates verified exports in declaration order.
    #[must_use]
    pub fn exports(&self) -> impl ExactSizeIterator<Item = VerifiedScalarExport<'_>> {
        self.exports
            .iter()
            .enumerate()
            .map(|(index, export)| VerifiedScalarExport { index, export })
    }

    /// Looks up one exact logical export.
    #[must_use]
    pub fn export(&self, logical_name: &str) -> Option<VerifiedScalarExport<'_>> {
        let index = *self.lookup.get(logical_name)?;
        Some(VerifiedScalarExport { index, export: &self.exports[index] })
    }

    /// Validates one typed invocation before target-specific encoding.
    ///
    /// # Errors
    ///
    /// Rejects an unknown export, wrong arity, or mismatched typed argument.
    pub fn prepare_invocation(
        &self,
        invocation: Invocation,
    ) -> Result<VerifiedInvocation<'_>, InvocationError> {
        let Some(export) = self.export(&invocation.logical_export) else {
            return Err(InvocationError::UnknownExport);
        };
        if export.parameters().len() != invocation.arguments.len() {
            return Err(InvocationError::ArityMismatch {
                expected: export.parameters().len(),
                actual: invocation.arguments.len(),
            });
        }
        for (index, (expected, actual)) in
            export.parameters().iter().zip(&invocation.arguments).enumerate()
        {
            if *expected != actual.ty() {
                return Err(InvocationError::TypeMismatch {
                    argument_index: index,
                    expected: *expected,
                    actual: actual.ty(),
                });
            }
        }
        Ok(VerifiedInvocation { export, arguments: invocation.arguments })
    }
}

/// Immutable view of one verified scalar export.
#[derive(Clone, Copy, Debug)]
pub struct VerifiedScalarExport<'module> {
    index: usize,
    export: &'module VerifiedExport,
}

impl<'module> VerifiedScalarExport<'module> {
    /// Returns the declaration index.
    #[must_use]
    pub const fn index(self) -> usize {
        self.index
    }

    /// Returns the verified logical export name.
    #[must_use]
    pub const fn logical_name(self) -> &'module LogicalExportName {
        &self.export.logical_name
    }

    /// Returns the verified JavaScript public export name.
    #[must_use]
    pub const fn javascript_name(self) -> &'module JavaScriptExportName {
        &self.export.javascript_name
    }

    /// Returns the verified core WebAssembly export name.
    #[must_use]
    pub const fn webassembly_name(self) -> &'module WebAssemblyExportName {
        &self.export.webassembly_name
    }

    /// Returns the verified Linux x86-64 symbol.
    #[must_use]
    pub const fn native_linux_x86_64_symbol(self) -> &'module NativeLinuxX8664Symbol {
        &self.export.native_linux_x86_64_symbol
    }

    /// Returns the verified parameter types.
    #[must_use]
    pub fn parameters(self) -> &'module [ScalarType] {
        &self.export.parameters
    }

    /// Returns the verified result type.
    #[must_use]
    pub const fn result(self) -> ScalarType {
        self.export.result
    }
}

/// Untrusted typed invocation request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Invocation {
    logical_export: String,
    arguments: Vec<ScalarValue>,
}

impl Invocation {
    /// Creates an invocation request for later module verification.
    #[must_use]
    pub const fn new(logical_export: String, arguments: Vec<ScalarValue>) -> Self {
        Self { logical_export, arguments }
    }
}

/// Stable typed invocation validation errors.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InvocationError {
    /// No exact logical export exists.
    UnknownExport,
    /// Argument count differs from the verified signature.
    ArityMismatch {
        /// Verified parameter count.
        expected: usize,
        /// Supplied argument count.
        actual: usize,
    },
    /// One argument carries the wrong scalar type.
    TypeMismatch {
        /// Zero-based argument index.
        argument_index: usize,
        /// Verified signature type.
        expected: ScalarType,
        /// Supplied value type.
        actual: ScalarType,
    },
}

impl InvocationError {
    /// Returns the stable machine-readable invocation error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::UnknownExport => "ZRYNA-B2101",
            Self::ArityMismatch { .. } => "ZRYNA-B2102",
            Self::TypeMismatch { .. } => "ZRYNA-B2103",
        }
    }
}

/// Invocation proven to match one verified scalar export.
#[derive(Clone, Debug)]
pub struct VerifiedInvocation<'module> {
    export: VerifiedScalarExport<'module>,
    arguments: Vec<ScalarValue>,
}

impl<'module> VerifiedInvocation<'module> {
    /// Returns the exact verified export.
    #[must_use]
    pub const fn export(&self) -> VerifiedScalarExport<'module> {
        self.export
    }

    /// Returns typed arguments in declaration order.
    #[must_use]
    pub fn arguments(&self) -> &[ScalarValue] {
        &self.arguments
    }
}

/// Verifies an untrusted module against scalar ABI v1.
///
/// # Errors
///
/// Returns deterministic bounded violations for invalid names, collisions, limits, or unsupported
/// signatures.
pub fn verify_v1(module: raw::Module) -> Result<VerifiedScalarAbiModule, Vec<AbiViolation>> {
    if module.exports.len() > MAX_ABI_EXPORTS {
        return Err(vec![AbiViolation {
            export_index: None,
            kind: AbiViolationKind::TooManyExports,
        }]);
    }

    let mut parameter_count = 0_usize;
    for (index, export) in module.exports.iter().enumerate() {
        if export.signature.parameters.len() > MAX_ABI_PARAMETERS_PER_EXPORT {
            return Err(vec![AbiViolation {
                export_index: Some(index),
                kind: AbiViolationKind::TooManyParameters,
            }]);
        }
        parameter_count =
            parameter_count.checked_add(export.signature.parameters.len()).ok_or_else(|| {
                vec![AbiViolation {
                    export_index: None,
                    kind: AbiViolationKind::TooManyParametersInModule,
                }]
            })?;
    }
    if parameter_count > MAX_ABI_PARAMETERS_PER_MODULE {
        return Err(vec![AbiViolation {
            export_index: None,
            kind: AbiViolationKind::TooManyParametersInModule,
        }]);
    }

    let mut errors = ViolationBuffer::default();
    let mut exact_names = BTreeMap::<Box<str>, usize>::new();
    let mut portable_names = BTreeMap::<Box<str>, usize>::new();
    let mut verified = Vec::with_capacity(module.exports.len());

    for (index, export) in module.exports.into_iter().enumerate() {
        if errors.exhausted() {
            break;
        }
        let parameters = export
            .signature
            .parameters
            .iter()
            .copied()
            .map(|ty| verify_type(index, ty, &mut errors))
            .collect::<Option<Vec<_>>>();
        let result = verify_type(index, export.signature.result, &mut errors);
        let mapping = verify_export_mapping(
            index,
            export.logical_name,
            &mut exact_names,
            &mut portable_names,
            &mut errors,
        );

        if let (Some(parameters), Some(result), Some(mapping)) = (parameters, result, mapping) {
            verified.push(VerifiedExport {
                logical_name: mapping.0,
                javascript_name: mapping.1,
                webassembly_name: mapping.2,
                native_linux_x86_64_symbol: mapping.3,
                parameters,
                result,
            });
        }
    }

    if !errors.is_empty() {
        return Err(errors.values);
    }

    let lookup = verified
        .iter()
        .enumerate()
        .map(|(index, export)| (export.logical_name.0.clone(), index))
        .collect();
    Ok(VerifiedScalarAbiModule { exports: verified, lookup })
}

fn verify_type(
    export_index: usize,
    ty: raw::Type,
    errors: &mut ViolationBuffer,
) -> Option<ScalarType> {
    match ty {
        raw::Type::Bool => Some(ScalarType::Bool),
        raw::Type::I32 => Some(ScalarType::I32),
        raw::Type::Unit => {
            errors.push(AbiViolation {
                export_index: Some(export_index),
                kind: AbiViolationKind::UnsupportedScalarType,
            });
            None
        }
    }
}

type VerifiedNames =
    (LogicalExportName, JavaScriptExportName, WebAssemblyExportName, NativeLinuxX8664Symbol);

fn verify_export_mapping(
    export_index: usize,
    name: String,
    exact_names: &mut BTreeMap<Box<str>, usize>,
    portable_names: &mut BTreeMap<Box<str>, usize>,
    errors: &mut ViolationBuffer,
) -> Option<VerifiedNames> {
    if !valid_logical_name(&name) {
        errors.push(AbiViolation {
            export_index: Some(export_index),
            kind: AbiViolationKind::InvalidLogicalName,
        });
        return None;
    }

    if let Some(first_index) = exact_names.get(name.as_str()).copied() {
        errors.push(AbiViolation {
            export_index: Some(export_index),
            kind: AbiViolationKind::DuplicateLogicalName { first_index },
        });
        return None;
    }
    exact_names.insert(name.clone().into_boxed_str(), export_index);

    let portable = name.to_ascii_lowercase().into_boxed_str();
    if let Some(first_index) = portable_names.get(portable.as_ref()).copied() {
        errors.push(AbiViolation {
            export_index: Some(export_index),
            kind: AbiViolationKind::PortableNameCollision { first_index },
        });
        return None;
    }
    portable_names.insert(portable, export_index);

    let native = format!("zryna_v1_e_{name}").into_boxed_str();
    let logical = name.into_boxed_str();
    Some((
        LogicalExportName(logical.clone()),
        JavaScriptExportName(logical.clone()),
        WebAssemblyExportName(logical),
        NativeLinuxX8664Symbol(native),
    ))
}

fn valid_logical_name(name: &str) -> bool {
    if name.is_empty() || name.len() > MAX_LOGICAL_EXPORT_NAME_BYTES {
        return false;
    }
    let mut bytes = name.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        && !reserved_export(name)
}

fn reserved_export(name: &str) -> bool {
    matches!(
        name,
        "arguments"
            | "await"
            | "break"
            | "case"
            | "catch"
            | "class"
            | "const"
            | "constructor"
            | "continue"
            | "debugger"
            | "default"
            | "delete"
            | "do"
            | "else"
            | "enum"
            | "eval"
            | "export"
            | "extends"
            | "false"
            | "finally"
            | "for"
            | "function"
            | "if"
            | "implements"
            | "import"
            | "in"
            | "instanceof"
            | "interface"
            | "let"
            | "new"
            | "null"
            | "package"
            | "private"
            | "protected"
            | "prototype"
            | "public"
            | "return"
            | "static"
            | "super"
            | "switch"
            | "then"
            | "this"
            | "throw"
            | "true"
            | "try"
            | "typeof"
            | "var"
            | "void"
            | "while"
            | "with"
            | "yield"
            | "__proto__"
    )
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    fn export(name: &str, parameters: Vec<raw::Type>, result: raw::Type) -> raw::Export {
        raw::Export::new(name.to_owned(), raw::Signature::new(parameters, result))
    }

    fn codes(errors: &[AbiViolation]) -> Vec<&str> {
        errors.iter().map(|error| error.code()).collect()
    }

    #[test]
    fn seals_exact_target_mappings_and_scalar_signatures() {
        let module = verify_v1(raw::Module::new(vec![
            export("add", vec![raw::Type::I32, raw::Type::I32], raw::Type::I32),
            export("negate", vec![raw::Type::Bool], raw::Type::Bool),
        ]))
        .expect("fixture ABI must verify");
        assert_eq!(module.version(), ScalarAbiVersion::V1);
        let exports = module.exports().collect::<Vec<_>>();
        assert_eq!(exports.len(), 2);
        assert_eq!(exports[0].logical_name().as_str(), "add");
        assert_eq!(exports[0].javascript_name().as_str(), "add");
        assert_eq!(exports[0].webassembly_name().as_str(), "add");
        assert_eq!(exports[0].native_linux_x86_64_symbol().as_str(), "zryna_v1_e_add");
        assert_eq!(exports[0].parameters(), &[ScalarType::I32, ScalarType::I32]);
        assert_eq!(exports[0].result(), ScalarType::I32);
        assert_eq!(exports[1].parameters(), &[ScalarType::Bool]);
        assert_eq!(exports[1].result(), ScalarType::Bool);
    }

    #[test]
    fn logical_names_are_bounded_reserved_and_portably_unique() {
        for invalid in ["", "1value", "value-name", "$value", "é", "default", "a\n"] {
            let errors =
                verify_v1(raw::Module::new(vec![export(invalid, Vec::new(), raw::Type::I32)]))
                    .expect_err("invalid logical export must fail");
            assert_eq!(codes(&errors), vec!["ZRYNA-B1001"], "name: {invalid:?}");
        }
        let exact = "a".repeat(MAX_LOGICAL_EXPORT_NAME_BYTES);
        assert!(
            verify_v1(raw::Module::new(vec![export(&exact, Vec::new(), raw::Type::I32)])).is_ok()
        );
        let too_long = "a".repeat(MAX_LOGICAL_EXPORT_NAME_BYTES + 1);
        assert_eq!(
            codes(
                &verify_v1(raw::Module::new(vec![export(&too_long, Vec::new(), raw::Type::I32,)]))
                    .expect_err("overlong logical export must fail")
            ),
            vec!["ZRYNA-B1001"]
        );

        let duplicate = verify_v1(raw::Module::new(vec![
            export("same", Vec::new(), raw::Type::I32),
            export("same", Vec::new(), raw::Type::I32),
        ]))
        .expect_err("exact duplicate must fail");
        assert_eq!(codes(&duplicate), vec!["ZRYNA-B1002"]);
        let collision = verify_v1(raw::Module::new(vec![
            export("valueName", Vec::new(), raw::Type::I32),
            export("valuename", Vec::new(), raw::Type::I32),
        ]))
        .expect_err("portable collision must fail");
        assert_eq!(codes(&collision), vec!["ZRYNA-B1003"]);
    }

    #[test]
    fn rejects_unsupported_signatures_and_resource_first_extras() {
        for malformed in [
            export("unitParameter", vec![raw::Type::Unit], raw::Type::I32),
            export("unitResult", Vec::new(), raw::Type::Unit),
        ] {
            assert_eq!(
                codes(
                    &verify_v1(raw::Module::new(vec![malformed]))
                        .expect_err("unit is not a scalar ABI v1 boundary type")
                ),
                vec!["ZRYNA-B1004"]
            );
        }

        let too_many_parameters = export(
            "parameters",
            vec![raw::Type::I32; MAX_ABI_PARAMETERS_PER_EXPORT + 1],
            raw::Type::I32,
        );
        assert_eq!(
            codes(
                &verify_v1(raw::Module::new(vec![too_many_parameters]))
                    .expect_err("first extra parameter must fail")
            ),
            vec!["ZRYNA-B1201"]
        );
        assert_eq!(
            codes(
                &verify_v1(raw::Module::new(vec![
                    export("f", Vec::new(), raw::Type::I32);
                    MAX_ABI_EXPORTS + 1
                ]))
                .expect_err("first extra export must fail")
            ),
            vec!["ZRYNA-B1201"]
        );

        let exact_parameters = (0..(MAX_ABI_PARAMETERS_PER_MODULE / MAX_ABI_PARAMETERS_PER_EXPORT))
            .map(|index| {
                export(
                    &format!("f{index}"),
                    vec![raw::Type::I32; MAX_ABI_PARAMETERS_PER_EXPORT],
                    raw::Type::I32,
                )
            })
            .collect();
        assert!(verify_v1(raw::Module::new(exact_parameters)).is_ok());
        let extra_parameters = (0..=(MAX_ABI_PARAMETERS_PER_MODULE
            / MAX_ABI_PARAMETERS_PER_EXPORT))
            .map(|index| {
                export(
                    &format!("f{index}"),
                    if index == MAX_ABI_PARAMETERS_PER_MODULE / MAX_ABI_PARAMETERS_PER_EXPORT {
                        vec![raw::Type::I32]
                    } else {
                        vec![raw::Type::I32; MAX_ABI_PARAMETERS_PER_EXPORT]
                    },
                    raw::Type::I32,
                )
            })
            .collect();
        assert_eq!(
            codes(
                &verify_v1(raw::Module::new(extra_parameters))
                    .expect_err("first extra aggregate parameter must fail")
            ),
            vec!["ZRYNA-B1201"]
        );
    }

    #[test]
    fn target_mappings_are_injective_and_independent_of_declaration_order() {
        let first = verify_v1(raw::Module::new(vec![
            export("alpha", Vec::new(), raw::Type::I32),
            export("beta", Vec::new(), raw::Type::I32),
        ]))
        .expect("fixture ABI must verify");
        let second = verify_v1(raw::Module::new(vec![
            export("beta", Vec::new(), raw::Type::I32),
            export("alpha", Vec::new(), raw::Type::I32),
        ]))
        .expect("reordered fixture ABI must verify");
        for name in ["alpha", "beta"] {
            let left = first.export(name).expect("first mapping");
            let right = second.export(name).expect("second mapping");
            assert_eq!(left.javascript_name(), right.javascript_name());
            assert_eq!(left.webassembly_name(), right.webassembly_name());
            assert_eq!(left.native_linux_x86_64_symbol(), right.native_linux_x86_64_symbol());
        }
        let names = first.exports().collect::<Vec<_>>();
        assert_ne!(names[0].javascript_name(), names[1].javascript_name());
        assert_ne!(names[0].webassembly_name(), names[1].webassembly_name());
        assert_ne!(names[0].native_linux_x86_64_symbol(), names[1].native_linux_x86_64_symbol());

        let maximum = "z".repeat(MAX_LOGICAL_EXPORT_NAME_BYTES);
        let module =
            verify_v1(raw::Module::new(vec![export(&maximum, Vec::new(), raw::Type::I32)]))
                .expect("maximum logical name must map");
        assert_eq!(
            module
                .exports()
                .next()
                .expect("one export")
                .native_linux_x86_64_symbol()
                .as_str()
                .len(),
            "zryna_v1_e_".len() + MAX_LOGICAL_EXPORT_NAME_BYTES
        );
    }

    #[test]
    fn violation_collection_is_bounded_and_terminal() {
        let invalid = (0..300)
            .map(|index| export(&format!("bad-{index}"), Vec::new(), raw::Type::I32))
            .collect();
        let errors = verify_v1(raw::Module::new(invalid)).expect_err("invalid names must fail");
        assert_eq!(errors.len(), MAX_ABI_VIOLATIONS);
        assert_eq!(errors.last().map(|error| error.code()), Some("ZRYNA-B1202"));
    }

    #[test]
    fn invocation_validation_preserves_full_scalar_types() {
        let module = verify_v1(raw::Module::new(vec![export(
            "choose",
            vec![raw::Type::Bool, raw::Type::I32],
            raw::Type::I32,
        )]))
        .expect("fixture ABI must verify");
        let prepared = module
            .prepare_invocation(Invocation::new(
                "choose".to_owned(),
                vec![ScalarValue::Bool(true), ScalarValue::I32(1)],
            ))
            .expect("typed invocation must verify");
        assert_eq!(prepared.export().logical_name().as_str(), "choose");
        assert_eq!(prepared.arguments(), &[ScalarValue::Bool(true), ScalarValue::I32(1)]);
        assert_eq!(
            module
                .prepare_invocation(Invocation::new("missing".to_owned(), Vec::new()))
                .expect_err("unknown export must fail"),
            InvocationError::UnknownExport
        );
        assert_eq!(
            module
                .prepare_invocation(Invocation::new("choose".to_owned(), Vec::new()))
                .expect_err("wrong arity must fail"),
            InvocationError::ArityMismatch { expected: 2, actual: 0 }
        );
        assert_eq!(
            module
                .prepare_invocation(Invocation::new(
                    "choose".to_owned(),
                    vec![ScalarValue::I32(1), ScalarValue::I32(1)],
                ))
                .expect_err("wrong typed argument must fail"),
            InvocationError::TypeMismatch {
                argument_index: 0,
                expected: ScalarType::Bool,
                actual: ScalarType::I32,
            }
        );
    }

    #[test]
    fn target_carriers_fail_closed_and_preserve_typed_outcomes() {
        for value in [i32::MIN, -1, 0, 1, i32::MAX] {
            assert_eq!(
                encode_argument(ScalarTarget::JavaScript, ScalarValue::I32(value)),
                RawHostScalar::JavaScriptNumber(f64::from(value))
            );
            assert_eq!(
                normalize_result(
                    ScalarTarget::JavaScript,
                    ScalarType::I32,
                    RawHostScalar::JavaScriptNumber(f64::from(value)),
                ),
                Ok(ScalarValue::I32(value))
            );
        }
        assert_eq!(
            encode_argument(ScalarTarget::JavaScript, ScalarValue::Bool(true)),
            RawHostScalar::JavaScriptBool(true)
        );
        for target in [ScalarTarget::CoreWebAssembly, ScalarTarget::NativeLinuxX8664] {
            assert_eq!(
                normalize_result(target, ScalarType::Bool, RawHostScalar::I32(0)),
                Ok(ScalarValue::Bool(false))
            );
            assert_eq!(
                decode_argument(target, ScalarType::Bool, RawHostScalar::I32(0)),
                Ok(ScalarValue::Bool(false))
            );
            assert_eq!(
                normalize_result(target, ScalarType::Bool, RawHostScalar::I32(1)),
                Ok(ScalarValue::Bool(true))
            );
            for invalid in [i32::MIN, -1, 2, i32::MAX] {
                assert_eq!(
                    normalize_result(target, ScalarType::Bool, RawHostScalar::I32(invalid)),
                    Err(ScalarBoundaryError::InvalidBoolCarrier(invalid))
                );
                assert_eq!(
                    decode_argument(target, ScalarType::Bool, RawHostScalar::I32(invalid)),
                    Err(ScalarBoundaryError::InvalidBoolCarrier(invalid))
                );
            }
        }
        for invalid in
            [f64::NEG_INFINITY, -0.0, 1.5, f64::from(i32::MAX) + 1.0, f64::NAN, f64::INFINITY]
        {
            assert!(matches!(
                normalize_result(
                    ScalarTarget::JavaScript,
                    ScalarType::I32,
                    RawHostScalar::JavaScriptNumber(invalid),
                ),
                Err(ScalarBoundaryError::InvalidJavaScriptI32(value)) if value.to_bits() == invalid.to_bits()
            ));
        }
        assert_eq!(
            normalize_result(
                ScalarTarget::JavaScript,
                ScalarType::Bool,
                RawHostScalar::JavaScriptNumber(1.0),
            ),
            Err(ScalarBoundaryError::TargetCarrierMismatch)
        );

        let integer = ScalarOutcome::Returned { value: ScalarValue::I32(1) };
        let boolean = ScalarOutcome::Returned { value: ScalarValue::Bool(true) };
        let zero = ScalarOutcome::Returned { value: ScalarValue::I32(0) };
        let false_value = ScalarOutcome::Returned { value: ScalarValue::Bool(false) };
        assert_ne!(integer, boolean);
        assert_ne!(zero, false_value);
        assert_ne!(
            serde_json::to_string(&integer).expect("serialize"),
            serde_json::to_string(&boolean).expect("serialize")
        );
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    struct FixtureDocument {
        schema_version: u32,
        abi: String,
        valid_exports: Vec<ValidExportFixture>,
        invalid_export_sets: Vec<InvalidExportFixture>,
        carrier_cases: Vec<CarrierFixture>,
        typed_outcomes: Vec<ScalarOutcome>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    struct ValidExportFixture {
        logical: String,
        parameters: Vec<FixtureType>,
        result: FixtureType,
        javascript: String,
        web_assembly: String,
        native_linux_x86_64: String,
    }

    #[derive(Clone, Copy, Debug, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    enum FixtureType {
        Bool,
        I32,
    }

    impl From<FixtureType> for raw::Type {
        fn from(value: FixtureType) -> Self {
            match value {
                FixtureType::Bool => Self::Bool,
                FixtureType::I32 => Self::I32,
            }
        }
    }

    impl From<FixtureType> for ScalarType {
        fn from(value: FixtureType) -> Self {
            match value {
                FixtureType::Bool => Self::Bool,
                FixtureType::I32 => Self::I32,
            }
        }
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    struct InvalidExportFixture {
        names: Vec<String>,
        expected_code: String,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    struct CarrierFixture {
        target: FixtureTarget,
        direction: FixtureDirection,
        scalar_type: FixtureType,
        raw: RawFixture,
        value: Option<ScalarValue>,
        error_code: Option<String>,
    }

    #[derive(Clone, Copy, Debug, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    enum FixtureTarget {
        #[serde(rename = "javascript")]
        JavaScript,
        #[serde(rename = "core-webassembly")]
        CoreWebAssembly,
        #[serde(rename = "native-linux-x86-64")]
        NativeLinuxX8664,
    }

    impl From<FixtureTarget> for ScalarTarget {
        fn from(value: FixtureTarget) -> Self {
            match value {
                FixtureTarget::JavaScript => Self::JavaScript,
                FixtureTarget::CoreWebAssembly => Self::CoreWebAssembly,
                FixtureTarget::NativeLinuxX8664 => Self::NativeLinuxX8664,
            }
        }
    }

    #[derive(Clone, Copy, Debug, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    enum FixtureDirection {
        EncodeArgument,
        DecodeArgument,
        NormalizeResult,
    }

    #[derive(Clone, Copy, Debug, Deserialize)]
    #[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
    enum RawFixture {
        #[serde(rename = "javascript-bool")]
        JavaScriptBool {
            value: bool,
        },
        #[serde(rename = "javascript-number")]
        JavaScriptNumber {
            number: JavaScriptNumberFixture,
        },
        I32 {
            value: i32,
        },
    }

    impl RawFixture {
        fn into_raw(self) -> RawHostScalar {
            match self {
                Self::JavaScriptBool { value } => RawHostScalar::JavaScriptBool(value),
                Self::JavaScriptNumber { number } => {
                    RawHostScalar::JavaScriptNumber(number.into_f64())
                }
                Self::I32 { value } => RawHostScalar::I32(value),
            }
        }
    }

    #[derive(Clone, Copy, Debug, Deserialize)]
    #[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
    enum JavaScriptNumberFixture {
        Finite { value: f64 },
        Nan,
        PositiveInfinity,
        NegativeInfinity,
    }

    impl JavaScriptNumberFixture {
        fn into_f64(self) -> f64 {
            match self {
                Self::Finite { value } => value,
                Self::Nan => f64::NAN,
                Self::PositiveInfinity => f64::INFINITY,
                Self::NegativeInfinity => f64::NEG_INFINITY,
            }
        }
    }

    fn raw_scalar_eq(left: RawHostScalar, right: RawHostScalar) -> bool {
        match (left, right) {
            (RawHostScalar::JavaScriptBool(left), RawHostScalar::JavaScriptBool(right)) => {
                left == right
            }
            (RawHostScalar::JavaScriptNumber(left), RawHostScalar::JavaScriptNumber(right)) => {
                left.to_bits() == right.to_bits()
            }
            (RawHostScalar::I32(left), RawHostScalar::I32(right)) => left == right,
            _ => false,
        }
    }

    fn parse_fixture(input: &str) -> Result<FixtureDocument, String> {
        let fixture =
            serde_json::from_str::<FixtureDocument>(input).map_err(|error| error.to_string())?;
        if fixture.schema_version != 1 || fixture.abi != "zryna-scalar-v1" {
            return Err("unsupported scalar ABI fixture version".to_owned());
        }
        Ok(fixture)
    }

    #[test]
    fn normative_fixture_is_byte_shared_and_matches_the_verifier() {
        const FIXTURE: &str = include_str!("../../../spec/abi/scalar-v1-fixtures.json");
        let fixture = parse_fixture(FIXTURE).expect("normative fixture must be strict JSON");

        for case in fixture.valid_exports {
            let module = verify_v1(raw::Module::new(vec![export(
                &case.logical,
                case.parameters.into_iter().map(Into::into).collect(),
                case.result.into(),
            )]))
            .expect("registered valid ABI fixture must verify");
            let mapped = module.exports().next().expect("one registered export");
            assert_eq!(mapped.javascript_name().as_str(), case.javascript);
            assert_eq!(mapped.webassembly_name().as_str(), case.web_assembly);
            assert_eq!(mapped.native_linux_x86_64_symbol().as_str(), case.native_linux_x86_64);
        }
        for case in fixture.invalid_export_sets {
            let exports =
                case.names.iter().map(|name| export(name, Vec::new(), raw::Type::I32)).collect();
            let errors = verify_v1(raw::Module::new(exports))
                .expect_err("registered invalid ABI fixture must fail");
            assert_eq!(errors[0].code(), case.expected_code);
        }
        for case in fixture.carrier_cases {
            let target = case.target.into();
            let expected_raw = case.raw.into_raw();
            match case.direction {
                FixtureDirection::EncodeArgument => {
                    let value = case.value.expect("encode fixture must provide a typed value");
                    assert!(case.error_code.is_none(), "encoding cannot declare an error");
                    assert_eq!(value.ty(), case.scalar_type.into());
                    assert!(
                        raw_scalar_eq(encode_argument(target, value), expected_raw),
                        "encoded carrier differs from the normative fixture"
                    );
                }
                FixtureDirection::DecodeArgument | FixtureDirection::NormalizeResult => {
                    let result = match case.direction {
                        FixtureDirection::DecodeArgument => {
                            decode_argument(target, case.scalar_type.into(), expected_raw)
                        }
                        FixtureDirection::NormalizeResult => {
                            normalize_result(target, case.scalar_type.into(), expected_raw)
                        }
                        FixtureDirection::EncodeArgument => unreachable!(),
                    };
                    match (case.value, case.error_code) {
                        (Some(value), None) => assert_eq!(result, Ok(value)),
                        (None, Some(code)) => {
                            assert_eq!(result.expect_err("invalid carrier").code(), code);
                        }
                        _ => panic!("boundary fixture must declare exactly one typed outcome"),
                    }
                }
            }
        }
        assert_eq!(fixture.typed_outcomes.len(), 4);
        assert_ne!(fixture.typed_outcomes[0], fixture.typed_outcomes[1]);
        assert!(matches!(fixture.typed_outcomes[2], ScalarOutcome::Trapped { .. }));
        assert!(matches!(fixture.typed_outcomes[3], ScalarOutcome::HostError { .. }));
    }

    #[test]
    fn normative_fixture_rejects_unknown_duplicate_and_version_drift() {
        const FIXTURE: &str = include_str!("../../../spec/abi/scalar-v1-fixtures.json");
        let unknown = FIXTURE.replacen('{', "{\"unknown\":true,", 1);
        assert!(parse_fixture(&unknown).is_err());
        let duplicate = FIXTURE.replacen(
            "\"schemaVersion\": 1,",
            "\"schemaVersion\":1,\"schemaVersion\":1,",
            1,
        );
        assert!(parse_fixture(&duplicate).is_err());
        let unknown_outcome = FIXTURE.replacen(
            "\"kind\": \"returned\",",
            "\"kind\":\"returned\",\"unknown\":true,",
            1,
        );
        assert!(parse_fixture(&unknown_outcome).is_err());
        let unknown_value =
            FIXTURE.replacen("\"type\": \"i32\",", "\"type\":\"i32\",\"unknown\":true,", 1);
        assert!(parse_fixture(&unknown_value).is_err());
        let unknown_trap =
            FIXTURE.replacen("\"kind\": \"trapped\",", "\"kind\":\"trapped\",\"unknown\":true,", 1);
        assert!(parse_fixture(&unknown_trap).is_err());
        let unknown_host_error = FIXTURE.replacen(
            "\"kind\": \"host-error\",",
            "\"kind\":\"host-error\",\"unknown\":true,",
            1,
        );
        assert!(parse_fixture(&unknown_host_error).is_err());
        let duplicate_outcome_kind = FIXTURE.replacen(
            "\"kind\": \"returned\",",
            "\"kind\":\"returned\",\"kind\":\"returned\",",
            1,
        );
        assert!(parse_fixture(&duplicate_outcome_kind).is_err());
        let duplicate_value_type =
            FIXTURE.replacen("\"type\": \"i32\",", "\"type\":\"i32\",\"type\":\"i32\",", 1);
        assert!(parse_fixture(&duplicate_value_type).is_err());
        let version = FIXTURE.replacen("\"schemaVersion\": 1", "\"schemaVersion\": 2", 1);
        assert!(parse_fixture(&version).is_err());
    }
}
