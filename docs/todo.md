# ToDo

- API Server:  TCP, REST (?)
  - Map(string,string)
    - use [Protocol Buffers](https://protobuf.dev/) for [wire messages](https://github.com/tokio-rs/prost)
    - set
    - get
    - e2e tests
    - benchmarks (single thread, thread pooling and async)

- API Client:  TCP, REST (?)
  - Map(string,string)
    - use [Protocol Buffers](https://protobuf.dev/) for [wire messages](https://github.com/tokio-rs/prost)
    - set
    - get
    - e2e tests
    - benchmarks (single thread, connection pooling and async)

- File Storage Service
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
