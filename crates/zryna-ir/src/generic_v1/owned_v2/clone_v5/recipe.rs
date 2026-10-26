//! Closed successor types form a prepaid DAG; runtime intervals count every occurrence.
use super::{
    CloneNodeKind, Edge, Failure, Node, TypeView, VerifiedLayouts, add, budget, plan, reject,
    reserve,
};
use zryna_layout::TypeCategory;
const ABSENT: u32 = u32::MAX;
pub(super) struct Builder<'a> {
    types: Vec<TypeView<'a>>,
    mapping: Vec<u32>,
    colors: Vec<u8>,
    pub(super) nodes: Vec<Node>,
}
struct Visit {
    ty: u32,
    node: u32,
    next: usize,
}
impl<'a> Builder<'a> {
    pub(super) fn new(layouts: &'a VerifiedLayouts, used: &mut usize) -> Result<Self, Failure> {
        let count = layouts.types().len();
        plan::charge(
            used,
            count.checked_mul(5).ok_or_else(|| budget("clone type inventory overflow"))?,
        )?;
        let mut types = reserve(count)?;
        types.extend(layouts.types());
        let mut mapping = reserve(count)?;
        mapping.resize(count, ABSENT);
        let mut colors = reserve(count)?;
        colors.resize(count, 0);
        Ok(Self { types, mapping, colors, nodes: Vec::new() })
    }
    pub(super) fn root(&mut self, ty: u32, used: &mut usize) -> Result<u32, Failure> {
        plan::charge(used, 1)?;
        if self.types.get(ty as usize).ok_or(Failure::InternalFailure)?.category()
            != TypeCategory::Struct
        {
            return Err(reject("CloneStruct source is not a nominal Struct"));
        }
        let mut stack = Vec::new();
        let root = self.enter(ty, &mut stack, used)?;
        while let Some(top) = stack.last() {
            let node = top.node;
            let source_ty = top.ty;
            if top.next < self.nodes[node as usize].edges.len() {
                plan::charge(used, 1)?;
                let next = top.next;
                stack.last_mut().ok_or(Failure::InternalFailure)?.next += 1;
                let child = self.nodes[node as usize].edges[next].child;
                if let Some(child) = child {
                    let target = self.enter(child, &mut stack, used)?;
                    self.nodes[node as usize].edges[next].child = Some(target);
                }
            } else {
                let mut strings =
                    u64::from(self.nodes[node as usize].kind == CloneNodeKind::String);
                let mut work = 1;
                for edge in 0..self.nodes[node as usize].edges.len() {
                    plan::charge(used, 1)?;
                    let child = self.nodes[node as usize].edges[edge].child;
                    work = add(work, 1)?;
                    if let Some(child) = child {
                        work = add(work, self.nodes[child as usize].work)?;
                    }
                    self.nodes[node as usize].edges[edge].start = strings;
                    if let Some(child) = child {
                        strings = add(strings, self.nodes[child as usize].strings)?;
                    }
                }
                self.nodes[node as usize].strings = strings;
                self.nodes[node as usize].work = work;
                self.colors[source_ty as usize] = 2;
                stack.pop();
            }
        }
        Ok(root)
    }
    fn enter(&mut self, ty: u32, stack: &mut Vec<Visit>, used: &mut usize) -> Result<u32, Failure> {
        plan::charge(used, 1)?;
        let color = *self.colors.get(ty as usize).ok_or_else(|| reject("clone type is absent"))?;
        if color == 1 {
            return Err(reject("clone layout contains a by-value cycle"));
        }
        if color == 2 {
            return Ok(self.mapping[ty as usize]);
        }
        if stack.len() >= zryna_layout::MAX_TRAVERSAL_DEPTH {
            return Err(budget("clone stored traversal depth"));
        }
        let view = *self.types.get(ty as usize).ok_or(Failure::InternalFailure)?;
        let kind = match view.category() {
            TypeCategory::Bool | TypeCategory::I32 => CloneNodeKind::Copy,
            TypeCategory::String => CloneNodeKind::String,
            TypeCategory::Struct | TypeCategory::Enum if view.drop_kind() == 0 => {
                CloneNodeKind::Copy
            }
            TypeCategory::Struct => CloneNodeKind::Struct,
            TypeCategory::Enum => CloneNodeKind::Enum,
            _ => return Err(reject("Struct clone child lacks this source execution proof")),
        };
        let edges = edges(view, kind, used)?;
        plan::charge(used, 2)?;
        self.nodes.try_reserve(1).map_err(|_| Failure::AllocationFailure)?;
        stack.try_reserve(1).map_err(|_| Failure::AllocationFailure)?;
        let node = u32::try_from(self.nodes.len()).map_err(|_| Failure::InternalFailure)?;
        self.nodes.push(Node { ty, kind, strings: 0, work: 0, edges });
        self.mapping[ty as usize] = node;
        self.colors[ty as usize] = 1;
        stack.push(Visit { ty, node, next: 0 });
        Ok(node)
    }
}
fn edges(view: TypeView<'_>, kind: CloneNodeKind, used: &mut usize) -> Result<Vec<Edge>, Failure> {
    let count = match kind {
        CloneNodeKind::Struct => view.fields().len(),
        CloneNodeKind::Enum => view.variants().len(),
        _ => 0,
    };
    plan::charge(used, count)?;
    let mut edges = reserve(count)?;
    match kind {
        CloneNodeKind::Struct => {
            let mut fields = view.fields();
            for _ in 0..count {
                plan::charge(used, 1)?;
                let (ordinal, ty, _) = fields.next().ok_or(Failure::InternalFailure)?;
                edges.push(Edge { ordinal, start: 0, child: Some(ty.index()) });
            }
        }
        CloneNodeKind::Enum => {
            let mut variants = view.variants();
            for _ in 0..count {
                plan::charge(used, 1)?;
                let (ordinal, child) = variants.next().ok_or(Failure::InternalFailure)?;
                edges.push(Edge {
                    ordinal,
                    start: 0,
                    child: child.map(zryna_layout::generic_v1::TypeId::index),
                });
            }
        }
        _ => {}
    }
    Ok(edges)
}
