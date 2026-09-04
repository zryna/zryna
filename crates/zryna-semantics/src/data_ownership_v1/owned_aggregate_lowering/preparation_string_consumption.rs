use super::super::super::super::Ty;
use super::super::super::preparation_plan::{Leaf, StringOperation, StringRead};
use zryna_ir::data_ownership_v1::raw;

struct OpenString {
    start: usize,
    kind: StringOperation,
    end: usize,
    ty: Ty,
    depth: usize,
    expected: Vec<StringRead>,
    reads: Vec<StringRead>,
    last: Option<raw::ValueId>,
}

#[derive(Default)]
pub(super) struct StringScopes {
    open: Vec<OpenString>,
    released: Option<OpenString>,
}

impl StringScopes {
    pub(super) fn start(&self) -> Option<usize> {
        self.open.last().map(|scope| scope.start)
    }
    pub(super) fn enter(
        &mut self,
        range: (usize, usize, usize),
        depth: usize,
        ty: Ty,
        kind: StringOperation,
        expected: Vec<StringRead>,
    ) {
        let (index, end, length) = range;
        assert!(self.released.is_none(), "String close must finish before another scope");
        assert!(end > index + 3 && end <= length, "String operation range");
        assert_eq!(
            expected.len(),
            if kind == StringOperation::Clone { 1 } else { 2 },
            "String operation exact arity"
        );
        if let Some(parent) = self.open.last() {
            assert!(end < parent.end, "nested String operation range");
        }
        self.open.push(OpenString {
            start: index,
            kind,
            end,
            ty,
            depth,
            expected,
            reads: Vec::new(),
            last: None,
        });
    }

    pub(super) fn read(&mut self, read: StringRead, ty: Ty) {
        let scope = self.open.last_mut().expect("String read has a scope");
        assert_eq!(scope.ty, ty, "String read exact type");
        assert_eq!(
            scope.expected.get(scope.reads.len()),
            Some(&read),
            "String read ordered role and identity"
        );
        if let Some(value) = read.value {
            assert_eq!(
                scope.last.take(),
                Some(value),
                "String read binds immediate produced value"
            );
        }
        scope.reads.push(read);
    }

    pub(super) fn exit(&mut self, index: usize, ty: Ty, depth: usize) {
        assert!(self.released.is_none(), "one released String operation");
        let scope = self.open.pop().expect("String exit has a scope");
        assert_eq!(
            (scope.end, scope.ty, scope.depth),
            (index + 3, ty, depth),
            "String exit exact range type and parent"
        );
        assert_eq!(scope.reads, scope.expected, "String operation complete ordered reads");
        self.released = Some(scope);
    }

    pub(super) fn leaf(&mut self, index: usize, ty: Ty, leaf: &Leaf<'_>) {
        let Some(scope) = self.released.take() else {
            assert!(!matches!(leaf, Leaf::StringConcat { .. }), "concat owns a String scope");
            return;
        };
        assert_eq!((scope.end, scope.ty), (index + 1, ty), "String result exact range and type");
        match (scope.kind, scope.reads.as_slice(), leaf) {
            (StringOperation::Clone, [read], Leaf::StringClone { source, bytes, .. }) => {
                assert_eq!(source.place, read.place, "String clone read linkage");
                assert_eq!(
                    (source.root, source.is_root, source.ty),
                    (read.root, read.place == read.root, ty),
                    "String clone authentic root and shape"
                );
                assert_eq!(*bytes, read.bytes, "String clone read byte linkage");
            }
            (
                StringOperation::Concat,
                [left, right],
                Leaf::StringConcat { left: actual_left, right: actual_right, bytes, .. },
            ) => {
                assert_eq!(
                    (*actual_left, *actual_right),
                    (left.place, right.place),
                    "String concat ordered read linkage"
                );
                let expected = match (left.bytes.known(), right.bytes.known()) {
                    (Some(left), Some(right)) => {
                        super::super::super::super::owned_string_read::StringBytes::Known(
                            left.checked_add(right).expect("prepared String sum"),
                        )
                    }
                    _ => super::super::super::super::owned_string_read::StringBytes::Unknown,
                };
                assert_eq!(expected, *bytes, "String concat exact byte fact");
            }
            _ => panic!("String result operation kind and arity"),
        }
    }

    pub(super) fn result(&mut self, value: raw::ValueId, depth: usize) -> bool {
        if let Some(scope) = self.open.last_mut() {
            scope.last = Some(value);
            return scope.depth != depth;
        }
        true
    }

    pub(super) fn pending_result(&self) -> bool {
        self.released.is_some()
    }

    pub(super) fn complete(&self) -> bool {
        self.open.is_empty() && self.released.is_none()
    }
}
