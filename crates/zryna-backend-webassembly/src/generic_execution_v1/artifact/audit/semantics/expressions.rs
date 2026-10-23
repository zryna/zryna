//! Small bounded address algebra, not an execution engine or a caller-supplied proof.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Expr {
    Unknown,
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
            Self::Const(n) => Self::AbsoluteLoad(n + offset.cast_signed(), width),
            Self::Local(base) => Self::Load(base, offset.cast_signed(), width),
            Self::Address(base, n) => Self::Load(base, n + offset.cast_signed(), width),
            _ => Self::Unknown,
        }
    }
    pub(super) fn add(self, right: Self) -> Self {
        match (self, right) {
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
            (Self::Local(i), Self::Const(n)) | (Self::Const(n), Self::Local(i)) => {
                Self::Scaled(i, n)
            }
            _ => Self::Unknown,
        }
    }
    pub(super) fn eq(self, right: Self) -> Self {
        match (self, right) {
            (Self::Local(2), Self::Const(n)) => Self::Pc(n),
            (Self::Load(base, offset, 4), Self::Const(n)) => Self::Tag(base, offset, n),
            _ => Self::Unknown,
        }
    }
}
