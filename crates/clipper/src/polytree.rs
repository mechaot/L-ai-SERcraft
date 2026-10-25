// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2010-2017 Angus Johnson (Clipper)
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)
// Clipper parts: Boost Software License 1.0, see NOTICE.md

//! Port of `PolyTree` / `PolyNode` from `Clipper/Clipper.cs`.
//! C# object graph becomes an arena: node 0 is the tree (root) itself, others are `PolyNode`s.

use crate::types::Path;

/// Handle of a node inside a `PolyTree` arena.
pub type NodeId = usize;

/// One node of the tree (`PolyNode`). Links are arena ids.
#[derive(Debug, Clone)]
pub struct PolyNode {
    pub(crate) parent: Option<NodeId>,
    pub(crate) polygon: Path,
    pub(crate) index: usize,
    pub(crate) childs: Vec<NodeId>,
    pub(crate) is_open: bool,
}

impl PolyNode {
    fn new() -> Self {
        PolyNode {
            parent: None,
            polygon: Vec::new(),
            index: 0,
            childs: Vec::new(),
            is_open: false,
        }
    }

    /// `ChildCount`. Returns number of children.
    pub fn child_count(&self) -> usize {
        self.childs.len()
    }

    /// `Contour`. Returns the node polygon / polyline.
    pub fn contour(&self) -> &Path {
        &self.polygon
    }

    /// `Childs`. Returns child ids in order.
    pub fn childs(&self) -> &[NodeId] {
        &self.childs
    }

    /// `Parent`. Returns parent id (root for top level nodes, `None` for the root itself).
    pub fn parent(&self) -> Option<NodeId> {
        self.parent
    }

    /// `IsOpen`. Returns true for open path results.
    pub fn is_open(&self) -> bool {
        self.is_open
    }
}

/// Result tree of `Execute` (`PolyTree`). Derives from `PolyNode` in C#: here the root is node 0.
#[derive(Debug, Clone)]
pub struct PolyTree {
    pub(crate) nodes: Vec<PolyNode>,
    pub(crate) all_polys: Vec<NodeId>,
}

impl Default for PolyTree {
    fn default() -> Self {
        Self::new()
    }
}

impl PolyTree {
    /// Id of the root (the tree itself).
    pub const ROOT: NodeId = 0;

    /// Empty tree.
    pub fn new() -> Self {
        PolyTree { nodes: vec![PolyNode::new()], all_polys: Vec::new() }
    }

    /// `Clear`: drop all nodes, keep root.
    pub fn clear(&mut self) {
        self.all_polys.clear();
        self.nodes.truncate(1);
        self.nodes[0].childs.clear();
    }

    /// `GetFirst`. Returns first top level node, if any.
    pub fn get_first(&self) -> Option<NodeId> {
        self.nodes[0].childs.first().copied()
    }

    /// `Total`. Returns node count (ignores the hidden outer rect of negative offsets).
    pub fn total(&self) -> usize {
        let mut result = self.all_polys.len();
        // negative offset leaves hidden outer polygon in all_polys
        if result > 0 && self.nodes[0].childs[0] != self.all_polys[0] {
            result -= 1;
        }
        result
    }

    /// Node access. `id` is a node id. Returns the node.
    pub fn node(&self, id: NodeId) -> &PolyNode {
        &self.nodes[id]
    }

    /// Root `ChildCount`. Returns number of top level nodes.
    pub fn child_count(&self) -> usize {
        self.nodes[0].childs.len()
    }

    /// Root `Childs`. Returns top level node ids.
    pub fn childs(&self) -> &[NodeId] {
        &self.nodes[0].childs
    }

    /// Allocate a detached node holding `polygon`. Returns its id.
    pub(crate) fn new_node(&mut self, polygon: Path) -> NodeId {
        let mut n = PolyNode::new();
        n.polygon = polygon;
        self.nodes.push(n);
        self.nodes.len() - 1
    }

    /// `AddChild`. `parent` gets `child` appended; child's parent and index updated.
    pub(crate) fn add_child(&mut self, parent: NodeId, child: NodeId) {
        let cnt = self.nodes[parent].childs.len();
        self.nodes[parent].childs.push(child);
        self.nodes[child].parent = Some(parent);
        self.nodes[child].index = cnt;
    }

    /// `IsHole`. `id` is the node. Returns true when nesting depth is odd.
    pub fn is_hole(&self, id: NodeId) -> bool {
        let mut result = true;
        let mut node = self.nodes[id].parent;
        while let Some(n) = node {
            result = !result;
            node = self.nodes[n].parent;
        }
        result
    }

    /// `GetNext`. `id` is the node. Returns next node in depth first order.
    pub fn get_next(&self, id: NodeId) -> Option<NodeId> {
        match self.nodes[id].childs.first() {
            Some(&c) => Some(c),
            None => self.get_next_sibling_up(id),
        }
    }

    /// `GetNextSiblingUp`. `id` is the node. Returns next sibling of node or of nearest ancestor.
    pub(crate) fn get_next_sibling_up(&self, id: NodeId) -> Option<NodeId> {
        let parent = self.nodes[id].parent?;
        if self.nodes[id].index == self.nodes[parent].childs.len() - 1 {
            self.get_next_sibling_up(parent)
        } else {
            Some(self.nodes[parent].childs[self.nodes[id].index + 1])
        }
    }
}
