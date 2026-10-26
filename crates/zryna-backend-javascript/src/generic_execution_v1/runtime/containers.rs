//! Private observed checkpoint carriers, using exact retained metadata and real allocator tokens.
//! The ambient test binding does not establish final module-private production runtime admission.
use super::super::{Context, Writer, context::charge, emission_budget};
use zryna_diagnostics::Diagnostic;
use zryna_ownership_runtime_abi::LogicalOperation as O;

pub(super) const VEC_OPERATIONS: [(&str, O); 2] =
    [("vecAlloc", O::VecAllocate), ("vecFree", O::VecReleaseStorage)];
pub(super) const CONTROL_OPERATIONS: [(&str, O); 7] = [
    ("controlAlloc", O::Allocate),
    ("strongClone", O::StrongClone),
    ("downgrade", O::WeakDowngrade),
    ("weakClone", O::WeakClone),
    ("strongBegin", O::StrongReleaseBegin),
    ("strongFinish", O::StrongReleaseFinish),
    ("weakFree", O::WeakRelease),
];
/// Independent helper families discovered from retained operation and loan roles.
#[derive(Clone, Copy)]
enum Helper {
    VecReserve,
    WeakUpgrade,
    StringConcat,
    LoanAddress,
}
impl Helper {
    const fn mask(self) -> u8 {
        match self {
            Self::VecReserve => 1,
            Self::WeakUpgrade => 2,
            Self::StringConcat => 4,
            Self::LoanAddress => 8,
        }
    }
}
#[derive(Clone, Copy)]
pub(super) struct Requirements(u8);
impl Requirements {
    fn include(&mut self, helper: Helper, required: bool) {
        if required {
            self.0 |= helper.mask();
        }
    }
    const fn has(self, helper: Helper) -> bool {
        self.0 & helper.mask() != 0
    }
    pub(super) fn additions(self) -> impl Iterator<Item = (&'static str, O)> {
        [
            self.has(Helper::VecReserve).then_some(("vecReserve", O::VecReserve)),
            self.has(Helper::WeakUpgrade).then_some(("upgrade", O::WeakUpgrade)),
            self.has(Helper::StringConcat).then_some(("concat", O::StringConcat)),
        ]
        .into_iter()
        .flatten()
    }
}
pub(super) fn requirements(c: &Context<'_, '_>) -> Result<Requirements, Diagnostic> {
    let operations = c.program.operations();
    charge(&c.work, operations.len().checked_add(c.functions.len()).ok_or_else(emission_budget)?)?;
    let mut requirements = Requirements(0);
    for operation in operations {
        requirements.include(Helper::VecReserve, operation.vec_push().is_some());
        requirements.include(
            Helper::StringConcat,
            matches!(
                operation.kind(),
                zryna_ir::generic_v1::execution_v1::raw::Operation::StringConcat { .. }
            ),
        );
    }
    for function in &c.functions {
        requirements.include(Helper::LoanAddress, !function.loans.is_empty());
        charge(&c.work, function.blocks.len())?;
        for block in &function.blocks {
            requirements.include(Helper::WeakUpgrade, block.weak_upgrade().is_some());
        }
    }
    Ok(requirements)
}
pub(super) fn declarations(
    c: &Context<'_, '_>,
    w: &mut Writer,
    requirements: Requirements,
) -> Result<(), Diagnostic> {
    if c.containers.has_vec {
        charge(&c.work, c.containers.elements.len())?;
        for (id, layout) in c.containers.elements.iter().enumerate() {
            let Some(layout) = layout else {
                continue;
            };
            w.format(format_args!("const $el{id}=Object.freeze(Object.assign(Object.create(null),{{id:{id},stride:{},alignment:{}}}));\n", layout.stride(), layout.alignment()))?;
        }
        w.trusted(if c.accounting.is_some() { VEC_INDEX } else { VEC })?;
    }
    if c.containers.has_control {
        charge(&c.work, c.containers.controls.len())?;
        for (id, layout) in c.containers.controls.iter().enumerate() {
            let Some(layout) = layout else {
                continue;
            };
            w.format(format_args!("const $ctl{id}=Object.freeze(Object.assign(Object.create(null),{{size:{},alignment:{},payloadOffset:{}}}));\n", layout.size(), layout.alignment(), layout.payload_offset()))?;
        }
        if c.accounting.is_none() {
            w.text(CONTROL)?;
        }
    }
    if requirements.has(Helper::VecReserve) && c.accounting.is_none() {
        w.trusted(PUSH)?;
    }
    if requirements.has(Helper::LoanAddress) {
        w.text(super::loans::HELPERS)?;
    }
    if requirements.has(Helper::StringConcat) && c.accounting.is_none() {
        w.text("function $stringConcat($left,$right) { return $handle($concat($left,$right),$status_concat); }\n")?;
    }
    Ok(())
}
const VEC: &str = r"function $vecNew($layout,$items,$initialized=$items.length) {
Object.setPrototypeOf($items,null);
const $out=Object.assign(Object.create(null),{$storage:0,$items:$items,$length:0,$dropParent:null,$dropType:-1,$dropIndex:-1});
const $result=$vecAlloc($layout,$items.length);
if (!$result || !Number.isInteger($result.status)) throw new Error('ZRYNA-RT-ABI');
$status_vecAlloc($result.status);
const $storage=$result.handle;
if ($items.length===0 ? $storage!==0 : !$storage || typeof $storage!=='object') throw new Error('ZRYNA-RT-ABI');
$out.$storage=$storage; $out.$length=$initialized; return $out;
}
function $vecIndex($vector,$index) {
if (($index|0)!==$index || $index<0 || $index>=$vector.$length) throw new Error('zryna.trap.bounds-v1');
return $vector.$items[$index];
}
";
const CONTROL: &str = r"function $controlNew($layout,$payload) {
const $control=Object.assign(Object.create(null),{$ptr:0,$strong:0,$weak:0,$pending:false,$payload:undefined,$dropParent:null,$dropType:-1,$dropIndex:-1});
const $result=$controlAlloc($layout.size,$layout.alignment);
if (!$result || !Number.isInteger($result.status)) throw new Error('ZRYNA-RT-ABI');
$status_controlAlloc($result.status);
const $pointer=$result.pointer;
if (!$pointer || typeof $pointer!=='object') throw new Error('ZRYNA-RT-ABI');
$control.$ptr=$pointer; $control.$strong=1; $control.$weak=1; $control.$payload=$payload; return $control;
}
function $countStrong($source) { $status_strongClone($strongClone($source)); return $source; }
function $countWeak($source) { $status_weakClone($weakClone($source)); return $source; }
function $countDowngrade($source) { $status_downgrade($downgrade($source)); return $source; }
function $boolResult($result,$status) {
if (!$result || !Number.isInteger($result.status)) throw new Error('ZRYNA-RT-ABI');
$status($result.status);
if (typeof $result.value!=='boolean') throw new Error('ZRYNA-RT-ABI');
return $result.value;
}
";

const PUSH: &str = r"function $vecPush($layout,$vector,$value) {
const $length=$vector.$length;
const $result=$vecReserve($layout,$vector.$storage,$length+1);
if (!$result || !Number.isInteger($result.status)) throw new Error('ZRYNA-RT-ABI');
$status_vecReserve($result.status);
const $storage=$result.handle;
if (!$storage || typeof $storage!=='object') throw new Error('ZRYNA-RT-ABI');
$vector.$storage=$storage;
$vector.$items[$length]=$value;
$vector.$length=$length+1;
}
";

const VEC_INDEX: &str = r"function $vecIndex($vector,$index) {
if (($index|0)!==$index || $index<0 || $index>=$vector.$length) throw new Error('zryna.trap.bounds-v1');
return $vector.$items[$index];
}
";
