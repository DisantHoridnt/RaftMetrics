# Distributed Analytics System

## Overview
This project is a basic distributed analytics system implemented in Rust. It demonstrates key distributed systems concepts such as fault tolerance, data partitioning, and distributed computation while maintaining a focused and concise design.

## libraries:
- `tokio`: Async runtime and concurrency.
- https://github.com/tikv/raft-rs
- `axum`: HTTP server implementation.
- `serde`: Data serialization and deserialization.
- `bincode`: Efficient binary data transmission.
- `tracing`: Logging and debugging.

## Features
- **Distributed Architecture**:
  - A single control node handles HTTP requests.
  - Two or more worker nodes manage partitions of the dataset using DuckDB.
- **Data Partitioning**:
  - Simple hash-based partitioning based on `partition_key`.
- **APIs**:
  - **Insert Endpoint**: Insert data into the appropriate node based on the partition key.
  - **Analytics Endpoint**: Compute the average of a value column across all nodes, even in the presence of failures.
- **Fault Tolerance**:
  - Partial results are returned if some nodes fail.
  - Concurrency control ensures safe writes during parallel operations.

## Technologies
- **Rust** (latest stable version)
- **tokio.rs**: For async runtime and concurrency.
- **axum**: For HTTP server implementation.
- **DuckDB**: Lightweight database for local storage.
- **Serde**: For data serialization and deserialization.
- **bincode** or **Protobuf**: For efficient binary data transmission.
- **tracing**: For logging and debugging.

## System Components
1. **Control Node**:
   - Handles HTTP requests for data insertion and analytics.
   - Routes requests to the appropriate worker node based on the partition key.

2. **Worker Nodes**:
   - Each node manages a DuckDB instance for its data partition.
   - Processes requests for insertion and local computation.

3. **Hash-based Partitioning**:
   - Uses a hash of `partition_key` to route data to the appropriate worker node.

## API Endpoints
### Insert Endpoint
- **Path**: `/insert`
- **Method**: POST
- **Input**: Binary format with fields:
  - `partition_key`: Unique key for partitioning.
  - `value`: Numeric value to be averaged.
- **Output**: Success or error response.

### Analytics Endpoint
- **Path**: `/analytics`
- **Method**: GET
- **Input**: None (or optional binary query parameters).
- **Output**: Binary response with:
  - `average`: The calculated average.
  - `is_partial`: Boolean indicating if the result is partial due to node failures.
  - `total_nodes`: Total nodes in the system.
  - `responding_nodes`: Number of nodes that responded.

## Running the System

### Using Docker Compose

```bash
# Build and start all services
docker-compose up --build

# Scale worker nodes (optional)
docker-compose up --scale worker=3
```

The system will start with:
- Control node on port 8080
- Worker1 on port 8081
- Worker2 on port 8082

### Manual Running
To run the services manually:

1. Start the control node:
```bash
cargo run --bin distributed_analytics_system
```

2. Start worker nodes:
```bash
cargo run --bin worker -- --id 1 --port 8081
cargo run --bin worker -- --id 2 --port 8082
```

## Project Structure
```
├── Cargo.toml           # Project dependencies and configuration
├── Dockerfile          # Multi-stage Docker build
├── docker-compose.yml  # Service orchestration
└── src/
    ├── main.rs         # Control node entry point
    ├── worker.rs       # Worker node entry point
    ├── lib.rs          # Library and partitioning logic
    ├── api/
    │   ├── mod.rs      # API module organization
    │   ├── control.rs  # Control node implementation
    │   └── worker.rs   # Worker node implementation
    └── raft/
        ├── mod.rs      # Raft module organization
        ├── node.rs     # Raft node setup
        └── storage.rs  # Raft storage implementation
```

## Development
- The system uses a multi-stage Dockerfile for optimal image size
- Docker Compose provides easy service orchestration
- Each service has logging enabled via `RUST_LOG=info`
- Hot reloading can be added for development
- Network isolation is handled via Docker networks

## Production Considerations
- The Docker setup is production-ready with minimal image size
- Easy to scale worker nodes horizontally
- Proper networking between services is configured
- Centralized logging is enabled
- Environment variables can be adjusted per deployment