use hashbrown::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, hash::Hash, ops::AddAssign};

/// A differential between two graphs.
///
/// Contains a diff for the nodes and edges of a graph. Each diff contains new or updated
/// items and items that are marked for deletion. The diff will always be internally consistent
/// if the safe, public methods are used.
///
/// `GraphDiff` requires two generic types:
/// * `Id` is the type used to index nodes in the graph. It requires standard trait bounds for
/// index types.
/// * `T` is the type used to represent node property updates. It requires `Default` used
/// when adding a new node to the diff and `AddAssign` to combine updates.
///
/// `GraphDiff`s support composition with `AddAssign`:
/// ```
/// use drisk_api::GraphDiff;
///
/// let mut diff1: GraphDiff<u32, u32> = GraphDiff::default();
/// let diff2: GraphDiff<u32, u32> = GraphDiff::default();
///
/// // `diff1` will contain all nodes and edges from `diff2`.
/// // If a node or edge is updated in both, the updates will be combined.
/// // Updates to the same properties from `diff2` will overwrite updates from `diff1`.
/// // If a node is deleted in `diff2`, it will be deleted in the combined diff.
/// diff1 += diff2;
/// ```
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct GraphDiff<Id: Hash + Eq + Copy, T: Default + AddAssign, W = f32> {
    pub(crate) nodes: NodeDiff<Id, T>,
    pub(crate) edges: EdgeDiff<Id, W>,
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

impl<Id: Hash + Eq + Copy, T: Default + AddAssign, W: Copy + PartialEq> GraphDiff<Id, T, W> {
    pub fn new() -> GraphDiff<Id, T> {
        GraphDiff::default()
    }

    /// Initialse diff from a NodeDiff and an EdgeDiff
    pub fn from_diffs(nodes: NodeDiff<Id, T>, edges: EdgeDiff<Id, W>) -> GraphDiff<Id, T, W> {
        GraphDiff { nodes, edges }
    }

    /// Get a reference to the node diff.
    pub fn nodes(&self) -> &NodeDiff<Id, T> {
        &self.nodes
    }

    /// Get a reference to the new or updated nodes.
    pub fn new_or_updated_nodes(&self) -> &HashMap<Id, T> {
        &self.nodes.new_or_updated
    }

    /// Get a reference to the deleted nodes.
    pub fn deleted_nodes(&self) -> &HashSet<Id> {
        &self.nodes.deleted
    }

    /// Get a reference to the edge diff.
    pub fn edges(&self) -> &EdgeDiff<Id, W> {
        &self.edges
    }

    /// Get a reference to the new or updated edges.
    pub fn new_or_updated_edges(&self) -> &HashMap<Id, HashMap<Id, W>> {
        &self.edges.new_or_updated
    }

    /// Get a reference to the deleted edges.
    pub fn deleted_edges(&self) -> &HashMap<Id, HashSet<Id>> {
        &self.edges.deleted
    }

    /// Returns `true` if the diff contains no nodes or edges (new, updated or deleted).
    pub fn is_empty(&self) -> bool {
        self.nodes.new_or_updated.is_empty()
            && self.nodes.deleted.is_empty()
            && self.edges.new_or_updated.is_empty()
            && self.edges.deleted.is_empty()
    }

    /// Add a new node to the diff. If previously marked as deleted, it will be overwritten.
    pub fn add_node(&mut self, node_id: &Id) {
        let _ = self.nodes.new_or_updated.try_insert(*node_id, T::default());
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

    /// Get a mutable reference to a node update in the diff. If the node is not
    /// present, it will be added with an empty update.
    pub fn get_or_create_mut_node_update(&mut self, node_id: &Id) -> &mut T {
        if self.nodes.new_or_updated.get(node_id).is_none() {
            self.add_node(node_id);
        };
        self.nodes.new_or_updated.get_mut(node_id).unwrap()
    }

    /// Use with caution: overwrites the node update to whatever you provide.
    pub fn set_node_update(&mut self, node_id: &Id, update: T) {
        self.nodes.new_or_updated.insert(*node_id, update);
        self.nodes.deleted.remove(node_id);
    }

    /// Add a new node to be deleted to the diff.
    /// If present the node will be removed from `new_or_updated`.
    /// It further updates the edge diff to make sure an edge
    /// deletion is recorded for all edges connecting to the node.
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

    /// Add a new edge to the diff.
    /// If previously marked as deleted, it will be overwritten
    /// If either the from or to nodes are marked as deleted, it will error.
    pub fn add_edge(
        &mut self,
        from: &Id,
        to: &Id,
        weight: W,
    ) -> Result<(), Box<dyn std::error::Error>> {
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

    /// Add edges in batch to the dif.
    pub fn add_edges(
        &mut self,
        edges: &HashMap<Id, HashMap<Id, W>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for (from, to_weight) in edges {
            for (to, weight) in to_weight {
                self.add_edge(from, to, *weight)?;
            }
        }
        Ok(())
    }

    /// Delete edges in batch from the diff.
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
    /// Does not check that the node IDs are valid (i.e. not marked as deleted).
    pub unsafe fn add_edges_unchecked(
        &mut self,
        edges: HashMap<Id, HashMap<Id, W>>,
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

    /// Add a new edge to be deleted to the diff.
    /// If present, the edge is removed from `new_or_updated`.
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

    /// Clear the diff of all nodes and edges.
    pub fn clear(&mut self) {
        self.nodes.new_or_updated.clear();
        self.nodes.deleted.clear();
        self.edges.new_or_updated.clear();
        self.edges.deleted.clear();
    }

    #[cfg(test)]
    fn is_internally_consistent(&self) -> bool {
        for (from, to_weight) in self.edges.new_or_updated.iter() {
            if self.nodes.deleted.contains(from) {
                return false;
            }
            for (to, _) in to_weight.iter() {
                if self.nodes.deleted.contains(to) {
                    return false;
                }
            }
        }
        for (from, to_set) in self.edges.deleted.iter() {
            if self.nodes.deleted.contains(from) {
                return false;
            }
            for to in to_set.iter() {
                if self.nodes.deleted.contains(to) {
                    return false;
                }
            }
        }
        true
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

/// A diff between the nodes of a graph.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
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

/// A diff between the edges of a graph.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EdgeDiff<Id: Hash + Eq, W = f32> {
    new_or_updated: HashMap<Id, HashMap<Id, W>>,
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

#[cfg(test)]
mod tests {

    use super::*;
    use crate::node_update::NodeUpdate;
    use hashbrown::HashMap;

    #[test]
    fn test_node() {
        let mut diff = GraphDiff::<usize, NodeUpdate>::new();

        let id = 1;
        let mut node = NodeUpdate {
            label: Some("test".to_string()),
            ..NodeUpdate::default()
        };

        diff.add_node(&id);
        diff.add_or_update_node(&id, node.clone());
        assert_eq!(diff.nodes.new_or_updated.get(&id).unwrap(), &node);

        node.size = Some(10.0);
        diff.add_or_update_node(&id, node.clone());
        assert_eq!(diff.nodes.new_or_updated.get(&id).unwrap(), &node);

        let node2 = NodeUpdate {
            green: Some(5),
            ..NodeUpdate::default()
        };
        diff.add_or_update_node(&id, node2.clone());

        let combined = NodeUpdate {
            label: Some("test".to_string()),
            size: Some(10.0),
            green: Some(5),
            ..NodeUpdate::default()
        };
        assert_eq!(diff.nodes.new_or_updated.get(&id).unwrap(), &combined);

        diff.delete_node(id);
        assert!(diff.nodes.new_or_updated.is_empty());
    }

    #[test]
    fn test_edge() {
        let mut diff = GraphDiff::<usize, NodeUpdate>::new();

        let from = 1;
        let to = 2;
        let weight = 1.0;

        diff.add_edge(&from, &to, weight).unwrap();
        assert_eq!(
            diff.edges
                .new_or_updated
                .get(&from)
                .unwrap()
                .get(&to)
                .unwrap(),
            &weight
        );

        let weight2 = 2.0;
        diff.add_edge(&from, &to, weight2).unwrap();
        assert_eq!(
            diff.edges
                .new_or_updated
                .get(&from)
                .unwrap()
                .get(&to)
                .unwrap(),
            &weight2
        );

        diff.delete_node(from);
        assert!(diff.edges.new_or_updated.is_empty());
    }

    #[test]
    fn test_add_assign_nodes() {
        let mut diff1 = GraphDiff::<usize, NodeUpdate>::new();
        let node = NodeUpdate {
            label: Some("test".to_string()),
            ..NodeUpdate::default()
        };
        let node_other = NodeUpdate {
            size: Some(10.0),
            ..NodeUpdate::default()
        };
        diff1.add_node(&1);
        diff1.add_or_update_node(&1, node.clone());
        diff1.add_node(&2);
        diff1.delete_node(3);

        let mut diff2 = GraphDiff::<usize, NodeUpdate>::new();
        diff2.add_node(&1);
        diff2.add_or_update_node(&1, node_other.clone());
        diff2.delete_node(2);

        diff1 += diff2;

        let d1 = diff1.nodes.new_or_updated.get(&1).unwrap();
        assert_eq!(d1.label.as_ref().unwrap(), "test");
        assert_eq!(d1.size.unwrap(), 10.0);
        assert!(!diff1.nodes.new_or_updated.contains_key(&2));
        assert!(diff1.nodes.deleted.contains(&2));
        assert!(diff1.nodes.deleted.contains(&3));
    }

    #[test]
    fn test_add_assign_edges() {
        let mut diff1 = GraphDiff::<usize, NodeUpdate>::new();
        diff1.add_edge(&1, &2, 1.0).unwrap();
        diff1.add_edge(&1, &3, 2.0).unwrap();
        diff1.add_edge(&1, &4, 2.0).unwrap();
        diff1.add_edge(&2, &3, 3.0).unwrap();
        diff1.add_edge(&3, &1, 4.0).unwrap();

        let mut diff2 = GraphDiff::<usize, NodeUpdate>::new();
        diff2.add_edge(&1, &2, 5.0).unwrap();
        diff2.add_edge(&2, &3, 6.0).unwrap();
        diff2.add_edge(&3, &1, 7.0).unwrap();
        diff2.delete_edge(&1, &3);

        diff1 += diff2;

        assert_eq!(
            diff1.edges.new_or_updated.get(&1).unwrap().get(&2).unwrap(),
            &5.0
        );
        assert_eq!(
            diff1.edges.new_or_updated.get(&2).unwrap().get(&3).unwrap(),
            &6.0
        );
        assert_eq!(
            diff1.edges.new_or_updated.get(&3).unwrap().get(&1).unwrap(),
            &7.0
        );
        assert_eq!(
            diff1.edges.new_or_updated.get(&1).unwrap().get(&4).unwrap(),
            &2.0
        );
        assert!(diff1.edges.deleted.get(&1).unwrap().contains(&3));
    }

    #[test]
    fn test_add_edges() {
        let mut diff = GraphDiff::<usize, usize>::new();
        for i in 0..50 {
            diff.add_node(&i);
        }

        for i in 10..20 {
            diff.delete_node(i);
        }

        let edges = (0..50usize)
            .map(|i| {
                let mut inner = HashMap::new();
                for j in 0..i {
                    inner.insert(j, 1f32);
                }
                (i, inner)
            })
            .collect::<HashMap<usize, HashMap<usize, f32>>>();

        // check can't add if nodes are deleted
        let mut diff2 = diff.clone();
        for i in 10..20 {
            diff2.delete_node(i);
        }
        assert!(diff2.add_edges(&edges).is_err());

        for i in 30..40 {
            diff.delete_node(i);
        }

        assert!(diff.is_internally_consistent());
    }
}
