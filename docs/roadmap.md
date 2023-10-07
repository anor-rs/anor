# Roadmap

- WIP: 1. Core Storage. Support for commonly used data types and structures
  - WIP: 1.1 Storing: file, blob, json, xml, arrays, set, map, primitive types
  - WIP: 1.2 Metafields: tags, descriptions, metadata
  
- WIP: 2. API Server (TCP)
  - WIP: 2.1 basic functionality
  - WIP: 2.2 async support

- WIP: 3. API Client (TCP)
  - (+)  3.1 basic functionality
  - WIP: 3.2 async support

- WIP: 4. File Storage Service (HTTP)
  - WIP: 4.1 basic functionality
  - WIP: 4.2 async support
  - WIP: 4.3 streaming (seeking and reading data at given position)
    - (+)  4.3.1 support for partial requests (Content-Range)
    - WIP: 4.3.2 support for If-Range, If-Modified-Since, If-None-Match, Last-Modified, Etag

- WIP: 5. Configuration
  - (+) 5.1 server and client sections

- WIP: 6. Logging
  - (+) 6.1 basic functionality

- 7. Authentification and Authorization
  - 7.1 support for JWT
  - 7.2 support for Fine-Grained Access Control

- 8. Remote collections
  - 8.1 HashSet
  - 8.2 HashMap
  - 8.3 Vec
  - 8.4 VecDeque

- 9. Scaling
  - easy scaling
    - support for built-in configuration service in nodes
    - support for auto replicating of configuration parameters in claster nodes
    - support for plug & play in adding a new node into claster and re-configurating of existing nodes
      - new node needs to know at least one neighbour in the claster. The added node and other nodes would be updated after completing the re-configuration and re-building process of claster.
    - any change in the claster configuration (adding new node, failing existing nodes), would be auto replicated to other nodes in the claster (no need for master node)
    - clients would be auto-updated after re-configuration
      - each client request includes a config_id parameter
      - server node will analyze the received config_id and may respond with updated configuration settings
    - clients and nodes use weighted graphs to optimize node/peer selection and other network operations.
  - consistent hashing
  - claster heartbeat

- 10. Caching
  - support for evictions (LRU)
  - support client-side and server-side caches

- 11. File Storage Spaces
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
