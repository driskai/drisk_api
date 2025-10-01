import os
from typing import Optional

import pytest

from drisk_api import GraphClient


@pytest.fixture
def auth_token() -> str:
    """Get the auth token from the environment."""
    token = os.getenv("CONODE_AUTH_TOKEN")
    if not token:
        raise ValueError("CONODE_AUTH_TOKEN environment variable is not set.")
    return token


@pytest.fixture
def server_url() -> Optional[str]:
    """Get the server url from the environment (Optional)."""
    return os.getenv("CONODE_SERVER_URL")


def test_api(auth_token, server_url):
    """Test the basic functionality of the GraphClient API."""
    graph = GraphClient.create_graph("test graph", auth_token, url=server_url)

    # make a new node
    node_id = graph.create_node(label="a node")

    # get node properties
    node = graph.get_node(node_id)
    assert node.properties["label"] == "a node"

    # get the successors of the node
    successors = graph.get_successors(node_id)
    assert len(successors) == 0

    # update the node
    graph.update_node(node_id, label="new label", size=3)

    # add edges in batch
    other_id = graph.create_node(label="another node")
    with graph.batch():
        graph.create_edge(node_id, other_id, weight=5.0)

    successors = graph.get_successors(node_id)
    assert len(successors) == 1
    assert successors[0] == other_id
