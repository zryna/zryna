//! Transport is minted only after source cleanup and both ordinary completion checks.
use super::super::{Context, Mode, Writer, flow, internal, runtime, shapes};
use zryna_abi::{ScalarType, ScalarValue, VerifiedInvocation};
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::execution_v1::invocation::{RECORD_END, RECORD_MAGIC, scalar_tag};
pub(super) fn module(
    c: &Context<'_, '_>,
    w: &mut Writer,
    mode: Mode,
    request: &VerifiedInvocation<'_>,
) -> Result<(), Diagnostic> {
    w.text(crate::prelude::JAVASCRIPT_PRELUDE)?;
    w.text("const $sourceTraps=Object.freeze([null,Object.freeze(Object.create(null)),Object.freeze(Object.create(null)),Object.freeze(Object.create(null)),Object.freeze(Object.create(null)),Object.freeze(Object.create(null))]);\nlet $pendingInvocation=null;\n")?;
    runtime::declarations(c, w, mode)?;
    shapes::declarations(c, w)?;
    for function in &c.functions {
        flow::function(c, function, w)?;
    }
    wrapper(c, w, request)
}
fn wrapper(
    c: &Context<'_, '_>,
    w: &mut Writer,
    request: &VerifiedInvocation<'_>,
) -> Result<(), Diagnostic> {
    let target =
        *c.program.export_functions().get(request.export().index()).ok_or_else(internal)?;
    let check = match request.export().result() {
        ScalarType::Bool => "$zryna$bool",
        ScalarType::I32 => "$zryna$i32",
    };
    w.text("function $invoke(){\n$pendingInvocation=null;\nif(arguments.length!==0)throw new Error('ZRYNA-INVOCATION-ARITY');\nif($busy)throw new Error('ZRYNA-RT-REENTRY');\n$busy=true;let $ctx,$status=0,$value=0;\ntry{try{$runtimeEnter();\n")?;
    if c.accounting.is_some() {
        w.text("$ctx=$accountStart();\n")?;
    }
    w.format(format_args!("try{{$value={check}($g{target}("))?;
    for (index, value) in request.arguments().iter().enumerate() {
        if index != 0 {
            w.text(",")?;
        }
        match value {
            ScalarValue::Bool(value) => w.text(if *value { "true" } else { "false" })?,
            ScalarValue::I32(value) => w.format(format_args!("{value}"))?,
        }
    }
    if c.accounting.is_some() {
        if !request.arguments().is_empty() {
            w.text(",")?;
        }
        w.text("$ctx")?;
    }
    w.text("));}catch($failure){\nif($failure===$sourceTraps[1])$status=1;else if($failure===$sourceTraps[2])$status=2;else if($failure===$sourceTraps[3])$status=3;else if($failure===$sourceTraps[4])$status=4;else if($failure===$sourceTraps[5])$status=5;else throw $failure;\n}\n}finally{$runtimeExit();")?;
    if c.accounting.is_some() {
        w.text("$accountExit($ctx);")?;
    }
    w.text("}}finally{$busy=false;}\n")?;
    let scalar = match request.export().result() {
        ScalarType::Bool => "($value?1:0)",
        ScalarType::I32 => "($value>>>0)",
    };
    w.format(format_args!("const $result=Object.freeze(Object.assign(Object.create(null),{{status:$status,type:{},scalar:$status===0?{scalar}:0}}));\n$pendingInvocation=$result;return $result;\n}}\n",scalar_tag(request.export().result())))?;
    w.format(format_args!("function $encode($result){{const $current=$pendingInvocation;$pendingInvocation=null;if(arguments.length!==1||$current===null||$result!==$current)throw new Error('ZRYNA-INVOCATION-IDENTITY');\nconst $bytes=new Uint8Array(20),$view=new DataView($bytes.buffer);$view.setUint32(0,{RECORD_MAGIC},true);$view.setUint32(4,$current.status,true);$view.setUint32(8,$current.type,true);$view.setUint32(12,$current.scalar,true);$view.setUint32(16,{RECORD_END},true);return $bytes;\n}}\nexport {{$invoke as {},$encode as {}}};\n",super::ENTRY,super::ENCODE))
}
