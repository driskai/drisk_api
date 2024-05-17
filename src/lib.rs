pub use crate::{
    bytes::{bytes_to_graph_diff, graph_diff_to_bytes},
    diff::{EdgeDiff, GraphDiff, NodeDiff},
    node_update::NodeUpdate,
};

mod bytes;
mod diff;
mod node_update;

#[cfg(feature = "extension-module")]
mod extension;
#[cfg(feature = "extension-module")]
pub use extension::*;
