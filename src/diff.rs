use hashbrown::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, hash::Hash, ops::AddAssign};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NodeDiff<Id: Hash + Eq, T> {
    new_or_updated: HashMap<Id, T>,
    deleted: HashSet<Id>,
}

impl<Id: Hash + Eq, T> NodeDiff<Id, T> {
    pub fn new(new_or_updated: HashMap<Id, T>, deleted: HashSet<Id>) -> NodeDiff<Id, T> {
        NodeDiff {
            new_or_updated,
            deleted,
        }
    }
    pub fn get_new_or_updated(&self) -> &HashMap<Id, T> {
        &self.new_or_updated
    }
    pub fn get_deleted(&self) -> &HashSet<Id> {
        &self.deleted
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EdgeDiff<Id: Hash + Eq> {
    new_or_updated: HashMap<Id, HashMap<Id, f32>>,
    deleted: HashMap<Id, HashSet<Id>>,
}

impl<Id: Hash + Eq> EdgeDiff<Id> {
    pub fn new(
        new_or_updated: HashMap<Id, HashMap<Id, f32>>,
        deleted: HashMap<Id, HashSet<Id>>,
    ) -> EdgeDiff<Id> {
        EdgeDiff {
            new_or_updated,
            deleted,
        }
    }
    pub fn get_new_or_updated(&self) -> &HashMap<Id, HashMap<Id, f32>> {
        &self.new_or_updated
    }
    pub fn get_deleted(&self) -> &HashMap<Id, HashSet<Id>> {
        &self.deleted
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GraphDiff<Id: Hash + Eq + Copy, T: Default + AddAssign> {
    pub nodes: NodeDiff<Id, T>,
    pub edges: EdgeDiff<Id>,
}

impl<Id: Hash + Eq + Copy, T: Default + AddAssign> Default for GraphDiff<Id, T> {
    fn default() -> GraphDiff<Id, T> {
        GraphDiff {
            nodes: NodeDiff {
                new_or_updated: HashMap::new(),
                deleted: HashSet::new(),
            },
            edges: EdgeDiff {
                new_or_updated: HashMap::new(),
                deleted: HashMap::new(),
            },
        }
    }
}

impl<Id: Hash + Eq + Copy, T: Default + AddAssign> GraphDiff<Id, T> {
    pub fn new() -> GraphDiff<Id, T> {
        GraphDiff::default()
    }

    pub fn new_or_updated_nodes(&self) -> &HashMap<Id, T> {
        &self.nodes.new_or_updated
    }

    pub fn deleted_nodes(&self) -> &HashSet<Id> {
        &self.nodes.deleted
    }

    pub fn new_or_updated_edges(&self) -> &HashMap<Id, HashMap<Id, f32>> {
        &self.edges.new_or_updated
    }

    pub fn deleted_edges(&self) -> &HashMap<Id, HashSet<Id>> {
        &self.edges.deleted
    }

    pub fn add_edges(
        &mut self,
        edges: &HashMap<Id, HashMap<Id, f32>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for (from, to_weight) in edges {
            for (to, weight) in to_weight {
                self.add_edge(from, to, *weight)?;
            }
        }
        Ok(())
    }

    pub fn delete_edges(
        &mut self,
        edges: &HashMap<Id, HashSet<Id>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for (from, to_set) in edges {
            for to in to_set {
                self.delete_edge(from, to);
            }
        }
        Ok(())
    }

    /// # Safety
    /// Does not check that the node IDs are valid.
    pub unsafe fn add_edges_unchecked(
        &mut self,
        edges: HashMap<Id, HashMap<Id, f32>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for (from, inner_map) in edges {
            self.edges
                .new_or_updated
                .entry(from)
                .or_default()
                .extend(inner_map);
        }
        Ok(())
    }

    /// Add a new node to the diff
    /// If previously marked as deleted, it will be overwritten.
    pub fn add_node(&mut self, node_id: &Id) {
        self.nodes.new_or_updated.insert(*node_id, T::default());
        self.nodes.deleted.remove(node_id);
    }

    /// Add or update a node in the diff with an update.
    /// If previously marked as deleted, it will be overwritten
    pub fn add_or_update_node(&mut self, node_id: &Id, update: T) {
        if let Some(node) = self.nodes.new_or_updated.get_mut(node_id) {
            *node += update;
        } else {
            self.nodes.new_or_updated.insert(*node_id, update);
        }
        self.nodes.deleted.remove(node_id);
    }

    pub fn get_or_create_mut_node_update(&mut self, node_id: &Id) -> &mut T {
        if self.nodes.new_or_updated.get(node_id).is_none() {
            self.add_node(node_id);
        };
        self.nodes.new_or_updated.get_mut(node_id).unwrap()
    }

    // Use with caution: sets the node update to whatever you provide.
    pub fn set_node_update(&mut self, node_id: &Id, update: T) {
        self.nodes.new_or_updated.insert(*node_id, update);
        self.nodes.deleted.remove(node_id);
    }

    /// Add a new edge to the diff
    /// if previously marked as deleted, it will be overwritten
    /// If either the from or to nodes are marked as deleted, it will error
    pub fn add_edge(
        &mut self,
        from: &Id,
        to: &Id,
        weight: f32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if weight.is_nan() || weight == std::f32::INFINITY || weight == std::f32::NEG_INFINITY {
            return Ok(()); // ignore invalid weights
        }

        if self.nodes.deleted.contains(from) || self.nodes.deleted.contains(to) {
            return Err("Either from or to nodes are marked to be deleted".into());
        }
        if let Some(inner) = self.edges.deleted.get_mut(from) {
            inner.remove(to);
        }
        if self.edges.deleted.get(from).is_some_and(|e| e.is_empty()) {
            self.edges.deleted.remove(from);
        }
        self.edges
            .new_or_updated
            .entry(*from)
            .or_default()
            .insert(*to, weight);
        Ok(())
    }

    /// Add to the track diff a new node to be deleted
    /// It removes such node from the new_or_updated list
    /// It further updates the edge diff to make sure an edge
    /// deletion is recorded for all edges connecting to such node
    pub fn delete_node(&mut self, node_id: Id) {
        self.nodes.new_or_updated.remove(&node_id);

        // remove all edges where node_id is predecessor
        self.edges.new_or_updated.remove(&node_id);

        for (from, to_weight) in self.edges.new_or_updated.iter_mut() {
            if to_weight.contains_key(&node_id) {
                self.edges.deleted.entry(*from).or_default().insert(node_id);
            }
            // remove all edges where node_id is successor
            to_weight.remove(&node_id);
        }
        self.nodes.deleted.insert(node_id);
    }

    /// Add to the track diff a new edge to be deleted
    /// It removes such edge from the new_or_updated list
    pub fn delete_edge(&mut self, from: &Id, to: &Id) {
        self.edges.deleted.entry(*from).or_default().insert(*to);

        let empty_inner_map = match self.edges.new_or_updated.get_mut(from) {
            None => false,
            Some(to_weight) => {
                to_weight.remove(to);
                to_weight.is_empty()
            }
        };
        if empty_inner_map {
            self.edges.new_or_updated.remove(from);
        }
    }

    pub fn clear(&mut self) {
        self.nodes.new_or_updated.clear();
        self.nodes.deleted.clear();
        self.edges.new_or_updated.clear();
        self.edges.deleted.clear();
    }
}

impl<Id: Hash + Eq + Copy, T: Default + AddAssign> AddAssign for GraphDiff<Id, T> {
    fn add_assign(&mut self, other: Self) {
        *self += other.nodes;
        *self += other.edges;
    }
}

impl<Id: Hash + Eq + Copy, T: Default + AddAssign> AddAssign<EdgeDiff<Id>> for GraphDiff<Id, T> {
    fn add_assign(&mut self, edges: EdgeDiff<Id>) {
        for (from, to_weight) in edges.new_or_updated {
            for (to, weight) in to_weight {
                let _ = self.add_edge(&from, &to, weight);
            }
        }
        for (from, to) in edges.deleted {
            for to in to {
                self.delete_edge(&from, &to);
            }
        }
    }
}

impl<Id: Hash + Eq + Copy, T: Default + AddAssign> AddAssign<NodeDiff<Id, T>> for GraphDiff<Id, T> {
    fn add_assign(&mut self, nodes: NodeDiff<Id, T>) {
        for (node_id, update) in nodes.new_or_updated {
            self.add_or_update_node(&node_id, update);
        }
        for node_id in nodes.deleted {
            self.delete_node(node_id);
        }
    }
}
