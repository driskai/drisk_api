"""Export graphs to networkx."""
try:
    import networkx as nx
except ImportError as e:
    raise ImportError(
        f"NetworkX is required for this module. Please install it.\n{e}"
    )

import json
from io import BytesIO
from zipfile import ZipFile

import requests

from drisk_api.graph_client import EdgeException, GraphClient


def graph_to_networkx(graph: GraphClient) -> nx.DiGraph:
    """Convert a graph to a NetworkX graph."""
    r = requests.get(
        f"{graph.url}/{graph.graph_id}/export-node-link",
        headers={"Authorization": graph.auth_token},
    )
    if r.status_code >= 300:
        raise EdgeException(r.status_code, r.text)
    zipped = ZipFile(BytesIO(r.content))
    unzipped = zipped.read(f"{graph.graph_id}_node_link.json").decode("utf-8")
    data = json.loads(unzipped)

    # apply default properties to nodes
    data["nodes"] = [{**graph.defaults, **n} for n in data["nodes"]]

    return nx.node_link_graph(data, multigraph=False, directed=True)
