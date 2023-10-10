# Roadmap

1. Data Storage
    - Storing: file, blob, json, xml, arrays, set, map, primitive types
    - Metafields: tags, descriptions, metadata
  
2. API Server (TCP)
    - basic functionality
    - async support

3. API Client (TCP)
    - basic functionality
    - async support

4. Configuration
   - server and client sections

5. Logging
    - basic logging

6. File Storage Service (HTTP)
    - basic functionality
    - async support
    - streaming (seeking and reading data at given position)
        - support for partial requests (Content-Range)
        - support for If-Range, If-Modified-Since, If-None-Match, Last-Modified, Etag

7. Authentification and Authorization
    - support for JWT
    - support for Fine-Grained Access Control

8. Remote collections
    - HashSet
    - HashMap
    - Vec
    - VecDeque

9. Scaling
    - cluster heartbeat
    - consistent hashing
    - easy scaling
      - support for built-in configuration service in nodes
      - support for auto replicating of configuration parameters in cluster nodes
      - support for plug & play in adding a new node into cluster and re-configurating of existing nodes
        - new node needs to know at least one neighbour in the cluster. The added node and other nodes would be updated after completing the re-configuration and re-building process of cluster
      - any change in the cluster configuration (adding new node, failing existing nodes), would be auto replicated to other nodes in the cluster (no need for master node)
      - clients would be notified and auto-updated after re-configuration
        - each client request includes a config_id parameter
        - server node will analyze the received config_id and may respond with updated configuration settings
      - clients and nodes use weighted graphs to optimize node/peer selection and other network operations
10. Caching
    - client-side and server-side caching support
    - cache evictions policies: LRU/LFU

11. File Storage Spaces
    - support for folders
    - support for path

## Notes

Storage space samples:

    path_id1 = path1/sub-path1
    path_id2 = path_id1/other_path2/item_id1

    get_item_type(path_id1) => PathItem
    parse_path(path_id1) => [
        path1 : FolderItem,
        subpath1 : FolderItem,
    ]

    get_item_type(path_id2) => PathItem
    parse_path(path_id2) => [
        path_id1 : PathItem,
        other_path2 : FolderItem,
        item_id1 : FileItem
    ]

    parse_full_path(path_id2) => [
        path1 : FolderItem,
        subpath1 : FolderItem,
        other_path2 : FolderItem,
        item_id1 : FileItem
    ]
