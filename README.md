# Edge Python API
API to connect to dRISK Edge.

## Installation
Build `rust` bindings and install:
```
pip install maturin
maturin develop -r --features extension-module
```

## Usage
```python
from edge_cli import GraphClient
```

Create a graph (requires auth token):
```python
graph = GraphClient.create_graph("a graph", token)
```

or connect to one
```python
graph = GraphClient("graph_id", token)
```

Create a node:
```python
node_id = graph.create_node(label="a node")
```

Get a node's successors:
```python
successors = graph.get_successors(node_id)
```

Update the properties of a node:
```python
graph.update_node(node_id, label="new label", size=3)
```

Make changes in batch:
```python
with graph.batch():
    graph.add_edge(node, other, weight=5.)
```
