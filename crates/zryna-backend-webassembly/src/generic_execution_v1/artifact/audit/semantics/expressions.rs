//! Small bounded address algebra, not an execution engine or a caller-supplied proof.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Expr {
    Unknown,
    IndirectLoad(u32, i64, i64, u8),
    IndirectDeref(u32, i64, i64, i64),
    IndirectScale(u32, i64, i64, i64),
    LoanVecElement(u32, i64, i64, i64, i64),
    LoanAppendFull(u32, i64, i64),
    LoanVecBounds(u32, i64, i64),
    LoanTag(u32, i64, i64, i64),
    LoanIndexed(u32, i64, i64, u32, i64),
    AppendFull(i64),
    UpgradeSuccess,
    UpgradeExpired,
    LoadedScale(u32, i64, i64),
    VecElement(u32, i64, i64, i64),
    CallResult(u32),
    PackedCallResult(u32),
    ControlZero(i64),
    ControlPair,
    NoOwners,
    GlobalDelta(u32, i64),
    Element(i64),
    Capacity(i64),
    Empty,
    Length,
    PackedStatus,
    UnsignedLocal(u32),
    AbsoluteLoad(i64, u8),
    AbsoluteDeref(i64, i64),
    Deref(u32, i64, i64),
    IndirectIndexed(u32, i64, u32, i64),
    VecBounds(u32, i64),
    Local(u32),
    Const(i64),
    Global(u32),
    Address(u32, i64),
    Scaled(u32, i64),
    Indexed(u32, i64, u32, i64),
    Load(u32, i64, u8),
    Tag(u32, i64, i64),
    Bounds(i64),
    Pc(i64),
}
impl Expr {
    pub(super) fn address(base: u32, offset: i64) -> Self {
        if offset == 0 { Self::Local(base) } else { Self::Address(base, offset) }
    }
    pub(super) fn load(self, offset: u64, width: u8) -> Self {
        match self {
            Self::Load(base, slot, 4) => {
                Self::IndirectLoad(base, slot, offset.cast_signed(), width)
            }
            Self::Deref(base, slot, delta) => {
                Self::IndirectLoad(base, slot, delta + offset.cast_signed(), width)
            }
            Self::Const(n) => Self::AbsoluteLoad(n + offset.cast_signed(), width),
            Self::Local(base) => Self::Load(base, offset.cast_signed(), width),
            Self::Address(base, n) => Self::Load(base, n + offset.cast_signed(), width),
            _ => Self::Unknown,
        }
    }
    pub(super) fn add(self, right: Self) -> Self {
        match (self, right) {
            // A decoded memarg0 must retain a previously proved loan-relative address.
            // Distinct descriptor field offsets remain distinct; no Unknown is authorized.
            (Self::Deref(base, slot, delta), Self::Const(n))
            | (Self::Const(n), Self::Deref(base, slot, delta)) => {
                let offset = delta + n;
                if offset == 0 {
                    Self::Load(base, slot, 4)
                } else {
                    Self::Deref(base, slot, offset)
                }
            }
            (Self::Load(base, pointer, 4), Self::IndirectScale(other, cell, length, stride))
                if base == other =>
            {
                Self::LoanVecElement(base, pointer, cell, length, stride)
            }
            (Self::IndirectLoad(base, cell, delta, 4), Self::Const(n))
            | (Self::Const(n), Self::IndirectLoad(base, cell, delta, 4)) => {
                Self::IndirectDeref(base, cell, delta, n)
            }
            (Self::IndirectLoad(base, cell, delta, 4), Self::Scaled(index, stride)) => {
                Self::LoanIndexed(base, cell, delta, index, stride)
            }
            (Self::Load(base, pointer, 4), Self::LoadedScale(other, length, stride))
            | (Self::LoadedScale(other, length, stride), Self::Load(base, pointer, 4))
                if base == other =>
            {
                Self::VecElement(base, pointer, length, stride)
            }
            (Self::Global(index), Self::Const(delta))
            | (Self::Const(delta), Self::Global(index)) => {
                if delta == 0 {
                    Self::Global(index)
                } else {
                    Self::GlobalDelta(index, delta)
                }
            }
            (Self::Load(base, slot, 4), Self::Const(n))
            | (Self::Const(n), Self::Load(base, slot, 4)) => {
                if n == 0 {
                    Self::Load(base, slot, 4)
                } else {
                    Self::Deref(base, slot, n)
                }
            }
            (Self::Load(base, slot, 4), Self::Scaled(index, stride))
            | (Self::Scaled(index, stride), Self::Load(base, slot, 4)) => {
                Self::IndirectIndexed(base, slot, index, stride)
            }
            (Self::AbsoluteLoad(slot, 4), Self::Const(n))
            | (Self::Const(n), Self::AbsoluteLoad(slot, 4)) => {
                if n == 0 {
                    Self::AbsoluteLoad(slot, 4)
                } else {
                    Self::AbsoluteDeref(slot, n)
                }
            }
            (Self::Indexed(base, offset, index, stride), Self::Const(n))
            | (Self::Const(n), Self::Indexed(base, offset, index, stride)) => {
                Self::Indexed(base, offset + n, index, stride)
            }
            (Self::Local(base), Self::Local(index)) => Self::Indexed(base, 0, index, 1),
            (Self::Local(base), Self::Const(n)) | (Self::Const(n), Self::Local(base)) => {
                Self::address(base, n)
            }
            (Self::Address(base, n), Self::Const(k)) | (Self::Const(k), Self::Address(base, n)) => {
                Self::address(base, n + k)
            }
            (Self::Local(base), Self::Scaled(index, stride))
            | (Self::Scaled(index, stride), Self::Local(base)) => {
                Self::Indexed(base, 0, index, stride)
            }
            (Self::Address(base, offset), Self::Scaled(index, stride))
            | (Self::Scaled(index, stride), Self::Address(base, offset)) => {
                Self::Indexed(base, offset, index, stride)
            }
            (Self::Const(a), Self::Const(b)) => Self::Const(a + b),
            _ => Self::Unknown,
        }
    }
    pub(super) fn sub(self, right: Self) -> Self {
        if let Self::Const(n) = right { self.add(Self::Const(-n)) } else { Self::Unknown }
    }
    pub(super) fn mul(self, right: Self) -> Self {
        match (self, right) {
            (Self::IndirectLoad(base, cell, delta, 4), Self::Const(n))
            | (Self::Const(n), Self::IndirectLoad(base, cell, delta, 4)) => {
                Self::IndirectScale(base, cell, delta, n)
            }
            (Self::Local(i), Self::Const(n)) | (Self::Const(n), Self::Local(i)) => {
                Self::Scaled(i, n)
            }
            (Self::Load(base, offset, 4), Self::Const(n))
            | (Self::Const(n), Self::Load(base, offset, 4)) => Self::LoadedScale(base, offset, n),
            _ => Self::Unknown,
        }
    }
    pub(super) fn eq(self, right: Self) -> Self {
        match (self, right) {
            (Self::Local(2), Self::Const(n)) => Self::Pc(n),
            (Self::Load(base, offset, 4), Self::Const(n)) => Self::Tag(base, offset, n),
            (Self::IndirectLoad(base, cell, delta, 4), Self::Const(n)) => {
                Self::LoanTag(base, cell, delta, n)
            }
            _ => Self::Unknown,
        }
    }
}
