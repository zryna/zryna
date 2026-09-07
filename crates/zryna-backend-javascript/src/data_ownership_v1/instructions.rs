use std::fmt::Write;

use zryna_ir::data_ownership_v1::{
    VerifiedBackendInstruction as B, VerifiedBackendTerminator as T, VerifiedCallArgument,
    VerifiedEdge, VerifiedFunction, VerifiedInstruction, VerifiedInstructionKind as K,
    VerifiedPlaceKind,
};
use zryna_layout::{TypeCategory, VerifiedLayouts};

use super::{error, format_error, private_name};

pub(super) fn emit_function(
    function: VerifiedFunction<'_>,
    layouts: &VerifiedLayouts,
    out: &mut impl Write,
) -> Result<(), zryna_diagnostics::Diagnostic> {
    write!(out, "function {}(", private_name(function)).map_err(format_error)?;
    let value_parameters = function.parameters().collect::<Vec<_>>();
    let borrow_parameters = function.borrow_parameters().collect::<Vec<_>>();
    for index in 0..value_parameters.len() + borrow_parameters.len() {
        if index > 0 {
            out.write_str(", ").map_err(format_error)?;
        }
        write!(out, "a{index}").map_err(format_error)?;
    }
    out.write_str(") {\n  const v = [], r = [], b = [], e = [];\n  const p = [")
        .map_err(format_error)?;
    let places = function.places().collect::<Vec<_>>();
    for (index, place) in places.iter().enumerate() {
        if index > 0 {
            out.write_char(',').map_err(format_error)?;
        }
        match place.kind() {
            VerifiedPlaceKind::Parameter(ordinal) => write!(out, "[0,{ordinal},0]"),
            VerifiedPlaceKind::Local(ordinal) => write!(out, "[1,{ordinal},0]"),
            VerifiedPlaceKind::Temporary(value) => write!(out, "[2,{},0]", value.index()),
            VerifiedPlaceKind::StructField { base, ordinal } => {
                write!(out, "[3,{},{}]", base.index(), ordinal)
            }
            VerifiedPlaceKind::EnumPayload { base, variant } => {
                write!(out, "[4,{},{}]", base.index(), variant)
            }
            VerifiedPlaceKind::FixedArrayConstant { base, index } => {
                write!(out, "[5,{},{}]", base.index(), index)
            }
        }
        .map_err(format_error)?;
    }
    out.write_str("];\n").map_err(format_error)?;
    writeln!(out, "  const cleanup = id => {{ $zryna$record({}); $zryna$record({}); $zryna$record(id); $zryna$drop($zryna$take(p,r,v,id)); }};",
        0x2000_0000_u32 + function.id().module(), function.id().declaration()).map_err(format_error)?;
    for (index, parameter) in value_parameters.iter().enumerate() {
        writeln!(out, "  v[{}] = a{index};", parameter.id().index()).map_err(format_error)?;
    }
    for place in &places {
        if let VerifiedPlaceKind::Parameter(ordinal) = place.kind() {
            writeln!(out, "  r[{}] = a{ordinal};", place.id().index()).map_err(format_error)?;
        }
    }
    for (index, parameter) in borrow_parameters.iter().enumerate() {
        writeln!(out, "  b[{}] = a{};", parameter.id().index(), value_parameters.len() + index)
            .map_err(format_error)?;
    }
    out.write_str("  let q = 0;\n  for (;;) {\n    switch (q) {\n").map_err(format_error)?;
    let blocks = function.blocks().collect::<Vec<_>>();
    let parameters =
        blocks.iter().map(|block| block.parameters().collect::<Vec<_>>()).collect::<Vec<_>>();
    for block in blocks {
        writeln!(out, "      case {}:", block.id().index()).map_err(format_error)?;
        for instruction in block.instructions() {
            emit_instruction(instruction, layouts, out)?;
        }
        emit_terminator(block.terminator(), &parameters, out)?;
    }
    out.write_str("      default: $zryna$trap(\"STATE\");\n    }\n  }\n}\n\n")
        .map_err(format_error)?;
    Ok(())
}

#[allow(clippy::too_many_lines)]
fn emit_instruction(
    instruction: VerifiedInstruction<'_>,
    layouts: &VerifiedLayouts,
    out: &mut impl Write,
) -> Result<(), zryna_diagnostics::Diagnostic> {
    let kind = instruction.kind();
    let data = instruction.backend_instruction();
    let failure_actions = if matches!(
        instruction.kind(),
        zryna_ir::data_ownership_v1::VerifiedInstructionKind::StructConstruct
            | zryna_ir::data_ownership_v1::VerifiedInstructionKind::EnumConstruct
            | zryna_ir::data_ownership_v1::VerifiedInstructionKind::FixedArrayConstruct
    ) {
        instruction.allocation_failure_drop_actions().collect::<Vec<_>>()
    } else {
        instruction.derived_drop_actions().collect::<Vec<_>>()
    };
    let catches = !failure_actions.is_empty()
        && !matches!(
            kind,
            K::InitializePlace
                | K::ReplacePlace
                | K::GenericReplacePlace
                | K::DropPlace
                | K::EndBorrow
        );
    if catches {
        out.write_str("        try { ").map_err(format_error)?;
    }
    let probes: &[u32] = match kind {
        K::StringFromUtf8 => &[5, 2],
        K::StringConcat | K::VecPush => &[3, 2],
        K::StructConstruct
        | K::FixedArrayConstruct
        | K::EnumConstruct
        | K::VecConstruct
        | K::SharedConstruct => &[2],
        K::WeakDowngrade => &[4],
        _ => &[],
    };
    for code in probes {
        write!(out, "$zryna$probe({code}); ").map_err(format_error)?;
    }
    if let Some(result) = instruction.result() {
        if !catches {
            out.write_str("        ").map_err(format_error)?;
        }
        write!(out, "v[{}] = ", result.index()).map_err(format_error)?;
    } else if !catches {
        out.write_str("        ").map_err(format_error)?;
    }
    let rendered: std::fmt::Result = (|| match (kind, data) {
        (K::BoolLiteral, B::BoolLiteral(value)) => {
            out.write_str(if value { "true" } else { "false" })
        }
        (K::I32Literal, B::I32Literal(value)) => write!(out, "{value}"),
        (K::I32Add, B::Binary(a, c)) => write!(out, "(v[{}] + v[{}]) | 0", a.index(), c.index()),
        (K::I32Sub, B::Binary(a, c)) => write!(out, "(v[{}] - v[{}]) | 0", a.index(), c.index()),
        (K::I32Mul, B::Binary(a, c)) => {
            write!(out, "$zryna$imul(v[{}], v[{}])", a.index(), c.index())
        }
        (K::I32Neg, B::Unary(value)) => write!(out, "(0 - v[{}]) | 0", value.index()),
        (K::Eq, B::Binary(a, c)) => write!(out, "v[{}] === v[{}]", a.index(), c.index()),
        (K::Ne, B::Binary(a, c)) => write!(out, "v[{}] !== v[{}]", a.index(), c.index()),
        (K::I32LtS, B::Binary(a, c)) => write!(out, "v[{}] < v[{}]", a.index(), c.index()),
        (K::I32LeS, B::Binary(a, c)) => write!(out, "v[{}] <= v[{}]", a.index(), c.index()),
        (K::I32GtS, B::Binary(a, c)) => write!(out, "v[{}] > v[{}]", a.index(), c.index()),
        (K::I32GeS, B::Binary(a, c)) => write!(out, "v[{}] >= v[{}]", a.index(), c.index()),
        (K::DirectCall, B::DirectCall { callee, arguments }) => {
            write!(out, "$zryna$d{}f{}(", callee.module(), callee.declaration())?;
            for (index, argument) in arguments.iter().enumerate() {
                if index > 0 {
                    out.write_str(", ")?;
                }
                match argument {
                    VerifiedCallArgument::Value(id) => write!(out, "v[{}]", id.index())?,
                    VerifiedCallArgument::Borrow(id) => write!(out, "b[{}]", id.index())?,
                }
            }
            out.write_char(')')
        }
        (K::StructConstruct | K::FixedArrayConstruct, B::Construct { operands, .. }) => {
            emit_sequence(&operands, "{$k:2,$v:[", "]}", out)
        }
        (K::EnumConstruct, B::Construct { operands, variant: Some(variant) }) => {
            write!(out, "{{$k:3,$t:{variant},$v:")?;
            if let Some(value) = operands.first() {
                write!(out, "v[{}]", value.index())?;
            } else {
                out.write_str("null")?;
            }
            out.write_char('}')
        }
        (K::CopyFromPlace, B::Place(place)) => emit_read(place.index(), out),
        (K::MoveFromPlace | K::GenericMoveFromPlace, B::Place(place)) => {
            write!(out, "$zryna$take(p,r,v,{})", place.index())
        }
        (
            K::ClonePlace
            | K::GenericClonePlace
            | K::HandleAwareClonePlace
            | K::StringClone
            | K::VecClone
            | K::SharedClone
            | K::WeakClone,
            B::Place(place),
        ) => {
            write!(out, "$zryna$clone($zryna$read(p,r,v,{}))", place.index())
        }
        (K::WeakDowngrade, B::Place(place)) => {
            write!(
                out,
                "(() => {{ const h=$zryna$read(p,r,v,{}); if(h.$c.$w===4294967295)$zryna$trap(\"REFCOUNT\"); h.$c.$w++; return {{$k:5,$c:h.$c}}; }})()",
                place.index()
            )
        }
        (K::DropPlace, B::Place(place)) => {
            write!(out, "cleanup({})", place.index())
        }
        (K::EnumDiscriminant, B::Place(place)) => {
            write!(out, "$zryna$read(p,r,v,{}).$t", place.index())
        }
        (
            K::InitializePlace | K::ReplacePlace | K::GenericReplacePlace,
            B::PlaceValue { place, value },
        ) => {
            if kind != K::InitializePlace {
                write!(out, "cleanup({}); ", place.index())?;
            }
            write!(out, "$zryna$write(p,r,v,{},v[{}])", place.index(), value.index())
        }
        (K::FixedArrayIndexCopy | K::VecIndexCopy, B::IndexedPlace { place, index }) => {
            write!(
                out,
                "(() => {{ const x=$zryna$read(p,r,v,{}).$v; return x[$zryna$index(v[{}],x.length)]; }})()",
                place.index(),
                index.index()
            )
        }
        (K::StringFromUtf8, B::String(bytes)) => {
            out.write_str("{$k:1,$v:")?;
            emit_string(bytes, out)?;
            out.write_char('}')
        }
        (K::StringConcat, B::StringConcat { left, right }) => write!(
            out,
            "$zryna$concat($zryna$read(p,r,v,{}),$zryna$read(p,r,v,{}))",
            left.index(),
            right.index()
        ),
        (K::VecConstruct, B::VecConstruct(values)) => {
            emit_sequence(&values, "{$k:2,$v:[", "]}", out)
        }
        (K::VecPush, B::VecPush { vector, value }) => {
            write!(out, "$zryna$push($zryna$read(p,r,v,{}),v[{}])", vector.index(), value.index())
        }
        (K::SharedConstruct, B::Unary(value)) => {
            write!(out, "{{$k:4,$c:{{$s:1,$w:1,$p:v[{}]}}}}", value.index())
        }
        (K::BeginBorrow, B::BeginBorrow(definition)) => write!(
            out,
            "b[{}]={{$p:{},$b:$zryna$U,$i:0}}",
            definition.id().index(),
            definition.place().index()
        ),
        (K::BeginIndexedBorrow | K::BeginIndexedAccess, B::IndexedBorrow { definition, index }) => {
            write!(
                out,
                "b[{}]={{$p:$zryna$U,$b:$zryna$U,$i:v[{}],$r:{}}}",
                definition.id().index(),
                index.index(),
                definition.place().index()
            )
        }
        (K::ProjectIndexedBorrow, B::ProjectIndexedBorrow { parent, borrow, index }) => write!(
            out,
            "b[{}]={{$p:$zryna$U,$b:{},$i:v[{}]}}",
            borrow.index(),
            parent.index(),
            index.index()
        ),
        (K::BindIndexedBorrow, B::BindIndexedBorrow { parent, borrow }) => {
            write!(out, "b[{}]=b[{}]", borrow.index(), parent.index())
        }
        (K::BorrowReplace | K::BorrowWrite, B::BorrowValue { borrow, value }) => {
            write!(out, "$zryna$borrowWrite(b,p,r,v,{},v[{}])", borrow.index(), value.index())
        }
        (K::BorrowRead, B::BorrowUse(borrow)) => {
            write!(out, "$zryna$borrowRead(b,p,r,v,{})", borrow.index())
        }
        (K::GenericCloneBorrow | K::HandleAwareCloneBorrow, B::BorrowUse(borrow)) => {
            write!(out, "$zryna$clone($zryna$borrowRead(b,p,r,v,{}))", borrow.index())
        }
        (K::EndBorrow, B::BorrowUse(borrow)) => write!(out, "b[{}]=$zryna$U", borrow.index()),
        _ => Err(std::fmt::Error),
    })();
    rendered.map_err(|_| error("ZRYNA-J3001", "verified instruction view could not be emitted"))?;
    out.write_str(";\n").map_err(format_error)?;
    if catches {
        out.write_str("        } catch ($failure) {\n").map_err(format_error)?;
        for action in failure_actions {
            writeln!(out, "          cleanup({});", action.root().index()).map_err(format_error)?;
        }
        out.write_str("          throw $failure;\n        }\n").map_err(format_error)?;
    }
    let _ = layouts;
    Ok(())
}

fn emit_terminator(
    terminator: zryna_ir::data_ownership_v1::VerifiedTerminator<'_>,
    parameters: &[Vec<zryna_ir::data_ownership_v1::VerifiedValueDefinition>],
    out: &mut impl Write,
) -> Result<(), zryna_diagnostics::Diagnostic> {
    match terminator.backend_terminator() {
        T::Return(value) => {
            emit_drops(terminator, out)?;
            writeln!(out, "        return v[{}];", value.index()).map_err(format_error)?;
        }
        T::Jump(edge) => emit_edge(&edge, &parameters[edge.target().index() as usize], 0, out)?,
        T::Branch { condition, when_true, when_false } => {
            writeln!(out, "        if (v[{}] === true) {{", condition.index())
                .map_err(format_error)?;
            emit_edge(&when_true, &parameters[when_true.target().index() as usize], 0, out)?;
            out.write_str("        } else {\n").map_err(format_error)?;
            emit_edge(&when_false, &parameters[when_false.target().index() as usize], 0, out)?;
            out.write_str("        }\n").map_err(format_error)?;
        }
        T::EnumMatch { place, arms } => {
            writeln!(out, "        switch ($zryna$read(p,r,v,{}).$t) {{", place.index())
                .map_err(format_error)?;
            for arm in arms {
                writeln!(out, "          case {}:", arm.variant()).map_err(format_error)?;
                let edge = arm.edge();
                emit_edge(edge, &parameters[edge.target().index() as usize], 0, out)?;
            }
            out.write_str("          default: $zryna$trap(\"ABI\");\n        }\n")
                .map_err(format_error)?;
        }
        T::WeakUpgrade { weak, success, expired } => {
            writeln!(out, "        {{ const c=$zryna$read(p,r,v,{}).$c;", weak.index())
                .map_err(format_error)?;
            out.write_str("          if (c.$s > 0) { try { $zryna$probe(4); if(c.$s===4294967295)$zryna$trap(\"REFCOUNT\"); } catch(failure) {\n").map_err(format_error)?;
            emit_drops(terminator, out)?;
            out.write_str("            throw failure; } c.$s++; const u={$k:4,$c:c};\n")
                .map_err(format_error)?;
            emit_edge(&success, &parameters[success.target().index() as usize], 1, out)?;
            out.write_str("          } else {\n").map_err(format_error)?;
            emit_edge(&expired, &parameters[expired.target().index() as usize], 0, out)?;
            out.write_str("          }\n        }\n").map_err(format_error)?;
        }
        T::Trap(identity) => {
            emit_drops(terminator, out)?;
            let name = match identity {
                zryna_ir::data_ownership_v1::VerifiedTrapIdentity::BoundsV1 => "BOUNDS",
                zryna_ir::data_ownership_v1::VerifiedTrapIdentity::AllocationV1 => "ALLOCATION",
                zryna_ir::data_ownership_v1::VerifiedTrapIdentity::CapacityV1 => "CAPACITY",
                zryna_ir::data_ownership_v1::VerifiedTrapIdentity::RefcountV1 => "REFCOUNT",
                zryna_ir::data_ownership_v1::VerifiedTrapIdentity::Utf8V1 => "UTF8",
            };
            writeln!(out, "        $zryna$trap(\"{name}\");").map_err(format_error)?;
        }
    }
    Ok(())
}

fn emit_edge(
    edge: &VerifiedEdge,
    parameters: &[zryna_ir::data_ownership_v1::VerifiedValueDefinition],
    synthesized: usize,
    out: &mut impl Write,
) -> Result<(), zryna_diagnostics::Diagnostic> {
    for (index, argument) in edge.arguments().enumerate() {
        writeln!(out, "            e[{index}]=v[{}];", argument.index()).map_err(format_error)?;
    }
    if synthesized == 1 {
        let first = parameters
            .first()
            .ok_or_else(|| error("ZRYNA-J3001", "weak success edge lacks synthesized parameter"))?;
        writeln!(out, "            v[{}]=u;", first.id().index()).map_err(format_error)?;
    }
    for (index, parameter) in parameters.iter().skip(synthesized).enumerate() {
        writeln!(out, "            v[{}]=e[{index}];", parameter.id().index())
            .map_err(format_error)?;
    }
    writeln!(out, "            q={}; continue;", edge.target().index()).map_err(format_error)?;
    Ok(())
}

fn emit_drops(
    terminator: zryna_ir::data_ownership_v1::VerifiedTerminator<'_>,
    out: &mut impl Write,
) -> Result<(), zryna_diagnostics::Diagnostic> {
    for action in terminator.derived_drop_actions() {
        writeln!(out, "        cleanup({});", action.root().index()).map_err(format_error)?;
    }
    Ok(())
}

fn emit_read(place: u32, out: &mut impl Write) -> std::fmt::Result {
    write!(out, "$zryna$read(p,r,v,{place})")
}

fn emit_sequence(
    values: &[zryna_ir::data_ownership_v1::ValueIdentity],
    prefix: &str,
    suffix: &str,
    out: &mut impl Write,
) -> std::fmt::Result {
    out.write_str(prefix)?;
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            out.write_char(',')?;
        }
        write!(out, "v[{}]", value.index())?;
    }
    out.write_str(suffix)
}

fn emit_string(bytes: &[u8], out: &mut impl Write) -> std::fmt::Result {
    out.write_char('"')?;
    for character in std::str::from_utf8(bytes).map_err(|_| std::fmt::Error)?.chars() {
        match character {
            '"' => out.write_str("\\\"")?,
            '\\' => out.write_str("\\\\")?,
            '\n' => out.write_str("\\n")?,
            '\r' => out.write_str("\\r")?,
            '\t' => out.write_str("\\t")?,
            c if c <= '\u{1f}' || c == '\u{2028}' || c == '\u{2029}' => {
                write!(out, "\\u{:04x}", u32::from(c))?;
            }
            c => out.write_char(c)?,
        }
    }
    out.write_char('"')
}

pub(super) fn emit_wrapper(
    function: VerifiedFunction<'_>,
    index: usize,
    name: &str,
    layouts: &VerifiedLayouts,
    out: &mut impl Write,
) -> Result<(), zryna_diagnostics::Diagnostic> {
    let parameters = function.parameters().collect::<Vec<_>>();
    write!(out, "function $zryna$de{index}(").map_err(format_error)?;
    for i in 0..parameters.len() {
        if i > 0 {
            out.write_str(", ").map_err(format_error)?;
        }
        write!(out, "a{i}").map_err(format_error)?;
    }
    out.write_str(") {\n").map_err(format_error)?;
    writeln!(out, "  $zryna$checkArity(arguments.length, {});", parameters.len())
        .map_err(format_error)?;
    for (i, parameter) in parameters.iter().enumerate() {
        let validator = validator(layouts, parameter.ty())?;
        writeln!(out, "  a{i}={validator}(a{i});").map_err(format_error)?;
    }
    let result = validator(layouts, function.result_type())?;
    write!(
        out,
        "  $zryna$status = 0; $zryna$trace = []; $zryna$attempt = 0;\n  try {{ return {result}({}(",
        private_name(function)
    )
    .map_err(format_error)?;
    for i in 0..parameters.len() {
        if i > 0 {
            out.write_str(", ").map_err(format_error)?;
        }
        write!(out, "a{i}").map_err(format_error)?;
    }
    writeln!(out, ")); }} catch (failure) {{ if (failure !== $zryna$sentinel) throw failure; return 0; }}\n}}\nexport {{ $zryna$de{index} as {name} }};\n").map_err(format_error)?;
    Ok(())
}

fn validator(
    layouts: &VerifiedLayouts,
    ty: zryna_layout::TypeId,
) -> Result<&'static str, zryna_diagnostics::Diagnostic> {
    match layouts.type_by_id(ty).map(zryna_layout::VerifiedType::category) {
        Some(TypeCategory::Bool) => Ok("$zryna$bool"),
        Some(TypeCategory::I32) => Ok("$zryna$i32"),
        _ => Err(error("ZRYNA-J3001", "public DataOwnershipV1 ABI is not scalar")),
    }
}
