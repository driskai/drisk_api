import io
from typing import Dict, Iterable, List, Optional, Tuple, Union
from uuid import UUID, uuid4

import requests

from .drisk_api import PyGraphDiff


class EdgeException(Exception):
    """General Edge Expection."""

    def __init__(self, status_code: int, message: str):
        message = f"Edge Server Error\nStatus Code: {status_code}\n{message}"
        super().__init__(message)


def edge_sync(func):
    """Sync edges."""

    def wrapper(self: "GraphClient", *args, **kwargs):
        result = func(self, *args, **kwargs)
        if self.batching is False:
            self._post_diff()
        return result

    return wrapper


class GraphClient:
    """A connection to a graph in Edge."""

    default_url = "http://localhost:5001/v3/graphs"
    defaults = {
        "label": "",
        "url": "",
        "size": 1.0,
        "red": 0,
        "green": 0,
        "blue": 0,
        "show_label": False,
    }

    @classmethod
    def create_graph(
        cls,
        graph_name: str,
        token: str,
        url: Optional[str] = None,
    ):
        """
        Create a new graph with the given name.

        Args:
            graph_name (str): Name of the graph.
            token (str): Authentication token.
            url (Optionalal[str]): API endpoint URL (default URL if not provided).

        Returns
        -------
            GraphClient: Instance representing the new graph.

        Raises
        ------
            EdgeException: If graph creation fails.

        """
        if url is None:
            url = cls.default_url

        r = requests.post(
            url,
            headers={"Authorization": token},
            params={"name": graph_name},
        )
        if not r.ok:
            raise EdgeException(r.text)
        graph_id = r.json()
        return cls(graph_id, token, url=url)

    def __init__(
        self,
        graph_id: UUID,
        auth_token: str,
        url: Optional[str] = None,
    ):
        self.url = url if url is not None else self.default_url
        if isinstance(graph_id, str):
            graph_id = UUID(graph_id)
        self.auth_token = auth_token
        self.diff = PyGraphDiff()
        self.graph_id = graph_id
        self.batch_count = 0
        self.connect()

    @property
    def batching(self):
        """Return `True` if the graph is in batching mode."""
        return self.batch_count > 0

    def connect(self):
        """
        Connect to the graph server and load the graph.

        Raises
        ------
            EdgeException: If connecting to the server or loading the graph fails.

        """
        url = f"{self.url}/{self.graph_id}/load"
        r = requests.get(url, headers={"Authorization": self.auth_token})
        if r.status_code >= 300:
            raise EdgeException(r.status_code, r.text)

    def rename_graph(self, name: str):
        """
        Rename a graph.

        Args:
            name (str): The new name for the graph.

        Raises
        ------
            ValueError: If the provided name is empty.
            EdgeException: If renaming the graph fails.

        """
        if len(name) == 0:
            raise ValueError("Name cannot be empty")
        r = requests.put(
            f"{self.url}/{self.graph_id}/save",
            headers={"Authorization": self.auth_token},
            params={"name": name, "groups": ""},
        )
        if r.status_code >= 300:
            raise EdgeException(r.status_code, r.text)

    def delete_graph(self):
        """
        Delete the graph.

        Raises
        ------
            EdgeException: If deleting the graph fails.

        """
        url = f"{self.url}/{self.graph_id}/delete"
        r = requests.delete(url, headers={"Authorization": self.auth_token})
        if r.status_code >= 300:
            raise EdgeException(r.status_code, r.text)

    def get_node(self, node_id: UUID) -> Optional["Node"]:
        """
        Get a node given the id.

        Args:
            node_id (UUID): The ID of the node.

        Returns
        -------
            Optional[Node]: The Node representation of the node_id. If it doesn't
            exist it will be none.

        """
        data = self._get_node_request(node_id)
        if data is None:
            return None
        return Node(
            self,
            node_id,
            **data["properties"],
        )

    def get_nodes(self, node_ids: List[UUID]) -> Dict[UUID, "Node"]:
        """
        Get multiple nodes given the ids.

        Args:
            node_ids (List[UUID]): The IDs of the nodes.

        Returns
        -------
            Dict[UUID, Node]: A mapping of node IDs to their Node representations.
            Any nodes that don't exist will not be included in the mapping.

        """
        url = f"{self.url}/{self.graph_id}/atomic/nodes"
        r = requests.post(
            url,
            headers={"Authorization": self.auth_token},
            json=[str(id) for id in node_ids],
        )
        if r.status_code >= 300:
            raise EdgeException(r.status_code, r.text)
        node_data = r.json()
        return {
            UUID(id): Node(self, UUID(id), **data["properties"])
            for id, data in node_data.items()
        }

    def get_successors(
        self,
        node_id: UUID,
        weights=False,
    ) -> Union[List[UUID], List[Tuple[UUID, float]]]:
        """
        Get the successors of a node.

        Args:
            node_id (UUID): The ID of the node.
            weights (bool, optional): Whether to include edge weights.
            Defaults to False.

        Returns
        -------
            Union[List[UUID], List[Tuple[UUID, float]]]: A List of successor node
            IDs, or a List of Tuples containing successor node IDs and their
            weights if `weights` is True.

        """
        return self._get_node_request(node_id, "successors", weights=weights)

    def get_predecessors(
        self,
        node_id: UUID,
        weights=False,
    ) -> Union[List[UUID], List[Tuple[UUID, float]]]:
        """
        Get the predecessors of a node.

        Args:
            node_id (UUID): The ID of the node.
            weights (bool, optional): Whether to include edge weights.
            Defaults to False.

        Returns
        -------
            Union[List[UUID], List[Tuple[UUID, float]]]: A List of predecessors node
            IDs, or a List of Tuples containing successor node IDs and their
            weights if `weights` is True.

        """
        return self._get_node_request(node_id, "predecessors", weights=weights)

    def get_edges(self, nodes: Iterable[UUID]) -> dict:
        """
        Get all edges between the given nodes.

        Args:
            nodes (Iterable[UUID]): Iterable of node IDs.

        Returns
        -------
            dict: Nested dictionary representing edges between nodes.
                Format: {from_node_id: {to_node_id: weight}}.
                Returns a dictionary for each given node even if it has no edges.

        """
        url = f"{self.url}/{self.graph_id}/atomic/edges"
        r = requests.post(
            url,
            headers={"Authorization": self.auth_token},
            json=[str(node) for node in nodes],
        )
        if r.status_code >= 300:
            raise EdgeException(r.status_code, r.text)
        return r.json()

    def create_view(
        self,
        label: str,
        x_node: Optional[str] = None,
        y_node: Optional[str] = None,
        filters: Optional[List[str]] = None,
    ) -> UUID:
        """
        Create a view node.

        Args:
            label (str): The label for the view node.
            x_node (Optionalal[str]): The label for the x-axis node.
            If None, a default x-axis node is created.
            y_node (Optionalal[str]): The label for the y-axis node.
            If None, a default y-axis node is created.
            filters (Optionalal[List[str]]): List of labels for filter nodes.

        Returns
        -------
            UUID: The ID of the created view node.

        """
        with self.batch():
            view_node = self.create_node(label=label)
            if x_node is None:
                x_node = self.create_node(label="x ")
            if y_node is None:
                y_node = self.create_node(label="y ")

            self.create_edge(self.graph_id, view_node, 1.0)
            self.create_edge(view_node, x_node, 0.0)
            self.create_edge(view_node, y_node, 1.0)

            if filters is not None:
                for i, f in enumerate(filters):
                    self.create_edge(view_node, f, i + 2.0)

        return view_node

    def get_view_axes_and_filters(
        self, view_node: str
    ) -> Tuple[str, str, List[str]]:
        """
        Get the x-axis, y-axis, and filter nodes of a view.

        Args:
        ----
            view_node (str): The ID of the view node.

        """
        x_node, y_node, *filters = [
            uuid
            for uuid, _ in sorted(
                zip(*self.get_successors(view_node, weights=True)),
                key=lambda x: x[1],
            )
        ]
        return x_node, y_node, filters

    def add_nodes_to_view(
        self, view_node: UUID, nodes: List[UUID], coords: List[Tuple]
    ):
        """
        Add nodes to a view with given coordinates.

        Args:
        ----
            view_node (str): The ID of the view node.
            nodes (List[str]): List of node IDs to add to the view.
            coords (List[Tuple]): List of coordinate Tuples (x, y) for each node.

        """
        x_node, y_node, *_ = [
            uuid
            for uuid, _ in sorted(
                zip(*self.get_successors(view_node, weights=True)),
                key=lambda x: x[1],
            )
        ]

        with self.batch():
            for n, (x, y) in zip(nodes, coords):
                self.create_edge(x_node, n, x)
                self.create_edge(y_node, n, y)

    def _post_diff(self):
        """
        Post the graph differences to the server.

        Raises
        ------
            EdgeException: If posting the graph differences fails.

        """
        if self._diff_size() == 0:
            return

        url = f"{self.url}/{self.graph_id}/graph-diff"
        r = requests.post(
            url,
            data=self.diff.to_bytes(),
            headers={"Authorization": self.auth_token},
        )
        if r.status_code >= 300:
            raise EdgeException(r.status_code, r.text)
        self.diff.clear()

    def _diff_size(self) -> int:
        """
        Calculate the size of the graph diff.

        Returns
        -------
            int: The total number of nodes and edges in the graph diff.

        """
        return self.diff.num_nodes() + self.diff.num_edges()

    def _get_node_request(
        self,
        node_id: UUID,
        nbr_type: Optional[str] = None,
        weights: bool = False,
    ) -> dict:
        """
        Retrieve information about a node from the server.

        Args:
            node_id (UUID): The ID of the node to retrieve information for.
            nbr_type (Optionalal[str]): The type of neighboring nodes to include
            (default: None).
            weights (bool): Include weights in the response (default: False).

        Returns
        -------
            dict: JSON response containing information about the node.

        Raises
        ------
            EdgeException: If retrieving information about the node fails.

        """
        url = f"{self.url}/{self.graph_id}/atomic/{node_id}"
        if nbr_type:
            query = f"?type={nbr_type}"
            if weights:
                query += "&weights=true"
            url += query
        r = requests.get(url, headers={"Authorization": self.auth_token})
        if r.status_code >= 300:
            raise EdgeException(r.status_code, r.text)
        return r.json()

    def batch(self) -> "Batch":
        """
        Enter the batching context.

        While in batching mode all changes to the graph are stored in memory
        and not communicated to the server. Note that performing read operations
        while in batching mode will not reflect the changes made in the batch so
        should be used with care.
        """
        return Batch(self)

    def set_batching_enabled(self, enabled: bool = True):
        """
        Enter batching mode.

        See `batch` method for more information.

        Args:
        ----
            enabled (bool, optional): Whether to enable batching (default: True).

        """
        self.batch_count += 1 if enabled else -1
        if not self.batching:
            self._post_diff()

    @edge_sync
    def create_node(self, label="node", **properties) -> UUID:
        """
        Create a new node with the given properties.

        Keyword Args:
            **properties: Properties of the node.

        Returns
        -------
            UUID: The ID of the created node.

        """
        kwargs = {**self.defaults, "label": label, **properties}
        id = kwargs.pop("id", uuid4())
        if isinstance(id, str):
            id = UUID(id)
        self.diff.add_node(id.bytes, kwargs)
        return id

    @edge_sync
    def create_edge(self, from_: UUID, to: UUID, weight: float = 1.0):
        """
        Create a new edge between two nodes.

        Args:
        ----
            from_ (UUID): The ID of the source node.
            to (UUID): The ID of the target node.
            weight (float, optional): The weight of the edge (default: 1.0).

        """
        if isinstance(from_, str):
            from_ = UUID(from_)
        if isinstance(to, str):
            to = UUID(to)
        self.diff.add_edge(from_.bytes, to.bytes, weight)

    @edge_sync
    def delete_node(self, node_id: UUID):
        """
        Delete a node from the graph.

        Args:
        ----
            node_id (UUID): The ID of the node to be deleted.

        """
        if isinstance(node_id, str):
            node_id = UUID(node_id)
        self.diff.delete_node(node_id.bytes)

    @edge_sync
    def delete_edge(self, from_: UUID, to: UUID):
        """
        Delete an edge between two nodes.

        Args:
        ----
            from_ (UUID): The ID of the source node.
            to (UUID): The ID of the target node.

        """
        if isinstance(from_, str):
            from_ = UUID(from_)
        if isinstance(to, str):
            to = UUID(to)
        self.diff.delete_edge(from_.bytes, to.bytes)

    @edge_sync
    def update_node(self, node_id: UUID, **new_properties):
        """
        Update properties of a node.

        Args:
        ----
            node_id (UUID): The ID of the node to update.
            **new_properties: Properties to update for the node.

        """
        if isinstance(node_id, str):
            node_id = UUID(node_id)
        self.diff.add_node(node_id.bytes, new_properties)

    def import_file_as_node(self, filename: str, file: io.BytesIO):
        """
        Import a file as a node in the graph.

        Args:
            filename (str): The name of the file.
            file (io.BytesIO): The file object to import.

        Returns
        -------
            str: The ID of the imported file node.

        Raises
        ------
            EdgeException: If importing the file fails.

        """
        url = f"{self.url}/{self.graph_id}/data/file"
        r = requests.post(
            url, headers={"Authorization": self.auth_token}, files={filename: file}
        )
        if not r.ok:
            raise EdgeException(r.status_code, r.text)

        file_id = r.json()

        return file_id

    def update_file_in_node(self, node_id: UUID, filename: str, file: io.BytesIO):
        """
        Update a file attached to a node in the graph.

        Args:
            node_id (UUID): The ID of the node containing the file.
            filename (str): The name of the file to update.
            file (io.BytesIO): The updated file object.

        Raises
        ------
            EdgeException: If updating the file fails.

        """
        url = f"{self.url}/{self.graph_id}/data/file/{node_id}"
        r = requests.put(
            url,
            headers={"Authorization": self.auth_token},
            files={filename: file},
        )
        if not r.ok:
            raise EdgeException(r.status_code, r.text)

    def add_data_from_json(self, filename: str, file: Union[Dict, List]) -> UUID:
        """
        Add data from a JSON file to the graph.

        Args:
            filename (str): The name of the JSON file.
            file (Union[Dict, List]): The JSON data to add.

        Returns
        -------
            str: The ID of the added JSON data.

        Raises
        ------
            EdgeException: If adding the JSON data fails.

        """
        url = f"{self.url}/{self.graph_id}/data/json"
        r = requests.post(
            url,
            headers={"Authorization": self.auth_token},
            json={"filename": filename, "file": file},
        )
        if not r.ok:
            raise EdgeException(r.status_code, r.text)

        json_id = r.json()

        return json_id


class Node:
    """Class representing a Node object."""

    def __init__(self, graph: GraphClient, id: UUID, **properties):
        """
        Initialize a Node instance.

        Args:
        ----
            graph (GraphClient): The graph client instance.
            id (UUID): The ID of the node.
            **properties: Additional properties of the node.

        """
        if isinstance(id, str):
            id = UUID(id)
        self.graph = graph
        self.id = id
        self._properties = {
            **graph.defaults,
            **properties,
        }

    def __repr__(self):
        return f"Node[{self.id}]"

    def __str__(self):
        return f"Node(label={self.label})"

    def update(self, **new_properties):
        """
        Update properties of the node.

        Args:
        ----
            **new_properties: New properties of the node.

        """
        self._properties = {**self._properties, **new_properties}
        self.graph.update_node(self.id, **self._properties)

    @property
    def properties(self) -> dict:
        """
        Get the properties of the node.

        Returns
        -------
            dict: The properties of the node.

        """
        return self._properties

    @properties.setter
    def properties(self):
        """Properties Setter. Prevent from using this."""
        raise ValueError("Cannot set properties. Use 'update_node' method.")

    @property
    def label(self) -> Optional[str]:
        """
        Get the label of the node.

        Returns
        -------
            str: The label of the node.

        """
        return self.properties.get("label")


class Batch:
    """Context Manager for batching graph operations for speed."""

    def __init__(self, graph: GraphClient):
        """
        Initialize Batch.

        Args:
        ----
            graph (GraphClient): The graph client instance.

        """
        self.graph = graph

    def __enter__(self):
        self.graph.batch_count += 1

    def __exit__(self, exc_type, exc_value, traceback):
        self.graph.batch_count -= 1
        if not self.graph.batching:
            self.graph._post_diff()
