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

## Testing with Postman

### 1. Insert Data (via Control Node)
```
Method: POST
URL: http://localhost:8080/insert
Headers: 
  Content-Type: application/json
Body:
{
    "partition_key": "user123",
    "value": 42.5
}
```

### 2. Insert Data (Worker 1)
```
Method: POST
URL: http://localhost:8081/insert
Headers: 
  Content-Type: application/json
Body:
{
    "partition_key": "user123",
    "value": 42.5
}
```

### 3. Insert Data (Worker 2)
```
Method: POST
URL: http://localhost:8082/insert
Headers: 
  Content-Type: application/json
Body:
{
    "partition_key": "user456",
    "value": 37.8
}
```

### 4. Compute Analytics
```
Method: POST
URL: http://localhost:8081/compute
Headers: 
  Content-Type: application/json
Response:
{
    "average": 42.5,
    "count": 1
}
```

### Testing Flow
1. Start the system:
   ```bash
   docker-compose up --build
   ```

2. Insert data through the control node:
   - This will automatically route data to the appropriate worker based on the partition key
   - You can verify the routing by checking compute results on different workers

3. Query individual workers:
   - Use the compute endpoint to see what data each worker has
   - This helps verify that the partitioning is working correctly

4. Expected Behavior:
   - Same partition_key should always route to the same worker
   - Different partition_keys may route to different workers
   - Each worker maintains its own data set
   - Compute results show only the data stored in that specific worker

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

# RaftMetrics: Distributed Analytics System

A robust, scalable distributed analytics platform built with Rust, implementing microservices architecture with Raft consensus and advanced monitoring capabilities.

## Features

### Core Functionality
- Distributed data processing with hash-based partitioning
- Raft consensus for reliable data replication
- Microservices architecture with control and worker nodes
- Persistent storage using DuckDB
- Dynamic service discovery
- RESTful API endpoints for data ingestion and querying

### Observability
- Structured logging with contextual information
- Prometheus metrics for system monitoring
- Health check endpoints
- Performance tracking and timing metrics

### Reliability
- Raft consensus for fault tolerance
- Leader election and failover
- Configuration change handling
- Comprehensive error handling
- Worker health monitoring

## Architecture

### Components
1. Control Node
   - Routes incoming metrics
   - Aggregates worker metrics
   - Monitors worker health
   - Manages Raft consensus
   - Handles service discovery

2. Worker Nodes
   - Store metrics data in DuckDB
   - Process local computations
   - Participate in Raft consensus
   - Provide health status

### Technology Stack
- **Language**: Rust (latest stable)
- **Web Framework**: Axum
- **Async Runtime**: Tokio
- **Database**: DuckDB
- **Consensus**: Raft
- **Logging**: Slog
- **Metrics**: Prometheus
- **Containerization**: Docker

## API Endpoints

### Control Node
```
POST /metrics      - Insert new metrics
GET  /aggregate    - Get aggregated metrics
GET  /health      - Check system health
```

### Worker Node
```
POST /insert      - Insert data point
POST /compute     - Compute local metrics
GET  /health      - Check worker health
```

## Metrics & Monitoring

### Node Metrics
- Node up/down status
- Leader status
- Storage size
- Operation counts

### Request Metrics
- Request counts
- Request durations
- Error rates
- Round-trip times

### Raft Metrics
- Proposal success/failure rates
- Election events
- Consensus timing
- State changes

## Configuration

### Environment Variables
```bash
# Required
NODE_ROLE=control|worker    # Node role
NODE_ID=<number>           # Unique node identifier
PORT=<number>              # Service port

# Optional
DB_PATH=/path/to/db        # DuckDB file path
WORKER_HOSTS=host1,host2   # Comma-separated worker hosts
CONTROL_HOST=host          # Control node address
```

## Development

### Prerequisites
- Rust 1.70+
- Docker & Docker Compose
- DuckDB

### Building
```bash
# Build the project
cargo build --release

# Run tests
cargo test

# Build Docker image
docker build -t raftmetrics .
```

### Running Locally
```bash
# Start the system
docker-compose up -d

# Scale workers
docker-compose up -d --scale worker=3
```

### Testing
```bash
# Unit tests
cargo test

# Integration tests
cargo test --test '*'

# Manual testing via Postman
# Import the provided collection: RaftMetrics.postman_collection.json
```

## Project Structure
```
├── Cargo.toml           # Dependencies
├── Dockerfile          # Multi-stage build
├── docker-compose.yml  # Service orchestration
└── src/
    ├── main.rs         # Entry point
    ├── lib.rs          # Core library
    ├── api/            # API implementations
    │   ├── control.rs  # Control node
    │   └── worker.rs   # Worker node
    ├── raft/           # Raft consensus
    │   ├── node.rs     # Raft node
    │   └── storage.rs  # Raft storage
    ├── logging.rs      # Logging setup
    └── metrics.rs      # Metrics tracking
```

## Error Handling
- Custom error types for different failure modes
- Contextual error reporting
- Structured error logging
- Proper error propagation

## Security Considerations
- Docker network isolation
- Environment-based configuration
- Minimal external exposure
- No hardcoded secrets

## Contributing
1. Fork the repository
2. Create your feature branch
3. Commit your changes
4. Push to the branch
5. Create a Pull Request

## License
MIT License - see LICENSE file for details

## Acknowledgments
- Rust community
- Raft consensus algorithm
- DuckDB team
- Axum framework