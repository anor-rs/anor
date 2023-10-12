# ToDo

- API: Map(string,string) operation (TCP, REST?)
  - research and analysis of
    - [Protocol Buffers](https://protobuf.dev/) implementation for Rust - [prost](https://github.com/tokio-rs/prost)
    - [MessagePack](https://msgpack.org/) implementation for Rust - [RMP](https://github.com/3Hren/msgpack-rust)
  - server: thread pooling (+ async)
  - client: connection pooling (+ async)
  - e2e tests
  - benchmarks (single thread, thread pooling and async)

- File Storage Service: HTTP
  - basic operations
    - upload
    - download
    - delete
  - streaming (seeking and reading data at given position)
    - support for partial requests (Content-Range)
    - support for If-Range, If-Modified-Since, If-None-Match, Last-Modified, Etag
  - e2e tests
  - benchmarks (single thread, pooling and async)

- Authorization:
  - node_token XOR …
  - client_access_token = …
  - node_token = …
  - path_access_token = …
  - item_access_token = …
