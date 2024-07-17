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
    """
    Convert a graph to a NetworkX `DiGraph`.

    Args:
        graph (GraphClient): The graph to convert.

    Returns
    -------
        nx.DiGraph
            The NetworkX graph.

    Raises
    ------
        EdgeException
            If the request fails.

    """
    r = requests.get(
        f"{graph.url}/{graph.graph_id}/export-node-link",
        headers={"Authorization": graph.auth_token},
    )
    if r.status_code >= 300:
        raise EdgeException(r.status_code, r.text)
    return node_link_to_nx(graph.graph_id, BytesIO(r.content))


def node_link_to_nx(graph_id, data_bytes) -> nx.DiGraph:
    """
    Convert raw node link data to a NetworkX `DiGraph`.

    Args:
        graph_id (str): The graph ID for zip extraction.

        data_bytes (BytesIO): The raw data bytes.

    Returns
    -------
        nx.DiGraph
            The NetworkX graph.

    """
    fname = f"{graph_id}_node_link.json"
    zipped = ZipFile(data_bytes)
    unzipped = zipped.read(fname).decode("utf-8")
    data = json.loads(unzipped)
    # apply default properties to nodes
    data["nodes"] = [{**GraphClient.defaults, **n} for n in data["nodes"]]
    return nx.node_link_graph(data, multigraph=False, directed=True)
