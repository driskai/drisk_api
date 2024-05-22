use crate::diff::{EdgeDiff, GraphDiff, NodeDiff};
use hashbrown::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use std::{hash::Hash, ops::AddAssign};

/*
 * GraphDiff (de-)serialization
 */

type SlimDiff<Id> = (
    HashMap<Id, String>, // JSON new node properties (serde field skip)
    HashSet<Id>,         // deleted node ids
    EdgeDiff<Id>,        // EdgeDiff
);

/// Serialize a `GraphDiff` to a byte vector.
pub fn graph_diff_to_bytes<Id, T>(
    diff: &GraphDiff<Id, T>,
) -> Result<Vec<u8>, Box<dyn std::error::Error>>
where
    Id: Copy + Eq + Hash + Serialize,
    T: AddAssign + Default + Serialize,
{
    // make use of serde skip fields
    let mut json_map: HashMap<Id, String> = HashMap::new();
    for (k, v) in diff.new_or_updated_nodes() {
        let json_str = serde_json::to_string(v)?;
        json_map.insert(*k, json_str);
    }
    Ok(bincode::serialize(&(
        json_map,
        diff.deleted_nodes(),
        diff.edges(),
    ))?)
}

/// Deserialize a `GraphDiff` from a byte slice.
pub fn bytes_to_graph_diff<Id, T>(
    bytes: &[u8],
) -> Result<GraphDiff<Id, T>, Box<dyn std::error::Error>>
where
    Id: Copy + Eq + Hash + for<'de> Deserialize<'de>,
    for<'a> T: AddAssign + Default + Deserialize<'a> + Serialize,
{
    let deserialized: SlimDiff<Id> = bincode::deserialize(bytes)?;
    let mut new_or_updated: HashMap<Id, T> = HashMap::new();
    for (id, json) in deserialized.0 {
        new_or_updated.insert(id, serde_json::from_str::<T>(&json)?);
    }
    Ok(GraphDiff {
        nodes: NodeDiff::new(new_or_updated, deserialized.1),
        edges: deserialized.2,
    })
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::{diff::GraphDiff, node_update::NodeUpdate};

    #[test]
    fn test_serialization() {
        let mut diff = GraphDiff::<usize, NodeUpdate>::new();
        diff.add_or_update_node(&1, NodeUpdate::default());
        diff.add_or_update_node(
            &2,
            NodeUpdate {
                label: Some("test".to_string()),
                ..NodeUpdate::default()
            },
        );
        diff.add_or_update_node(
            &3,
            NodeUpdate {
                red: Some(5),
                ..NodeUpdate::default()
            },
        );
        diff.add_edge(&1, &2, 0.).unwrap();
        diff.add_edge(&2, &3, 10.).unwrap();

        let bytes = graph_diff_to_bytes(&diff).unwrap();
        let deserialized = bytes_to_graph_diff::<usize, NodeUpdate>(&bytes).unwrap();
        assert_eq!(diff, deserialized);
    }
}
