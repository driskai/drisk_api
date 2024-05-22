//! Graph diff library for Rust and extension module for Python.
//!
//! A graph diff is a delta between two graphs. This module provides:
//! * a generic graph diff implentation,
//! * a serialization/deserialization API,
//! * specific types for the dRISK API,
//! * and a Python extension module for the dRISK API.
//!
//! See the documentation for `GraphDiff` for more information.
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
