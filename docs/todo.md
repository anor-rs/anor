# ToDo

- API: Map(string,string) operation (TCP, REST?)
  - research and analysis of
    - [Protocol Buffers](https://protobuf.dev/) for [wire messages](https://github.com/tokio-rs/prost)
    - [MessagePack](https://msgpack.org/)
  - server: thread pooling, async
  - client: connection pooling, async
  - e2e tests
  - benchmarks (single thread, thread pooling and async)

- File Storage Service: HTTP
  - upload
  - download
  - delete
  - complete the full support for Range requests
    - HTTP if-range
  - e2e tests
  - benchmarks (single thread, pooling and async)

- Authorization:
  - node_token XOR …
  - client_access_token = …
  - node_token = …
  - path_access_token = …
  - item_access_token = …
