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

## API Reference

### Authentication

All API endpoints require authentication using a Bearer token:

```http
Authorization: Bearer your-auth-token
```

Rate limiting is applied per token:
- 1000 requests per minute for control node
- 5000 requests per minute for worker nodes

### Query Parameters

#### Time Range Parameters
- `from`: Start timestamp (ISO 8601)
- `to`: End timestamp (ISO 8601)
- `window`: Time window for aggregation (e.g., "1h", "1d")

Examples:
```http
# Last 24 hours
GET /aggregate?metric=cpu_usage&window=1h

# Custom range with specific timestamps
GET /aggregate?metric=cpu_usage&from=2024-01-20T00:00:00Z&to=2024-01-20T23:59:59Z

# Rolling window
GET /aggregate?metric=cpu_usage&window=1h&rolling=true
```

#### Filtering Parameters
- `tags`: Filter by metadata tags
- `hosts`: Filter by specific hosts
- `datacenters`: Filter by datacenters

Examples:
```http
# Filter by datacenter
GET /aggregate?metric=cpu_usage&datacenters=us-west,us-east

# Filter by tags
GET /aggregate?metric=cpu_usage&tags=environment:prod,service:api

# Filter by hosts with wildcards
GET /aggregate?metric=cpu_usage&hosts=web-*,api-*
```

#### Aggregation Parameters
- `agg`: Aggregation functions (avg, min, max, sum, count, p50, p95, p99)
- `group_by`: Group results by field
- `order_by`: Sort results
- `limit`: Limit number of results

Examples:
```http
# Multiple aggregations
GET /aggregate?metric=cpu_usage&agg=avg,max,p95

# Group by datacenter and host
GET /aggregate?metric=cpu_usage&group_by=datacenter,host

# Top 10 hosts by CPU usage
GET /aggregate?metric=cpu_usage&group_by=host&order_by=value:desc&limit=10
```

### Control Node API

#### Insert Metrics
```http
POST /metrics
Content-Type: application/json

{
    "key": "cpu_usage",
    "value": 75.5,
    "timestamp": "2024-01-20T15:30:00Z",
    "tags": {
        "host": "server-1",
        "datacenter": "us-west"
    }
}
```

Response:
```json
{
    "success": true,
    "message": "Metric inserted successfully",
    "worker_node": "worker-2"
}
```

#### Get Aggregated Metrics
```http
GET /aggregate?metric=cpu_usage&from=2024-01-20T00:00:00Z&to=2024-01-20T23:59:59Z
```

Response:
```json
{
    "metric": "cpu_usage",
    "aggregations": {
        "avg": 68.5,
        "min": 45.2,
        "max": 92.1,
        "count": 1440,
        "p95": 85.3
    },
    "by_datacenter": {
        "us-west": {
            "avg": 70.2,
            "count": 720
        },
        "us-east": {
            "avg": 66.8,
            "count": 720
        }
    }
}
```

#### Health Check
```http
GET /health
```

Response:
```json
{
    "status": "healthy",
    "uptime": "2d 15h 30m",
    "worker_nodes": {
        "total": 3,
        "healthy": 3
    },
    "raft_status": {
        "role": "leader",
        "term": 5,
        "last_log_index": 1205
    }
}
```

### Worker Node API

#### Insert Data
```http
POST /insert
Content-Type: application/json

{
    "partition_key": "cpu_usage",
    "value": 75.5,
    "timestamp": "2024-01-20T15:30:00Z",
    "metadata": {
        "host": "server-1",
        "datacenter": "us-west"
    }
}
```

Response:
```json
{
    "success": true,
    "partition": 2,
    "storage_size": "1.2GB"
}
```

#### Compute Local Metrics
```http
POST /compute
Content-Type: application/json

{
    "metric": "cpu_usage",
    "aggregations": ["avg", "max", "p95"],
    "time_window": {
        "from": "2024-01-20T00:00:00Z",
        "to": "2024-01-20T23:59:59Z"
    },
    "group_by": ["datacenter"]
}
```

Response:
```json
{
    "results": {
        "avg": 68.5,
        "max": 92.1,
        "p95": 85.3
    },
    "groups": {
        "us-west": {
            "avg": 70.2,
            "max": 92.1,
            "p95": 86.1
        }
    },
    "computation_time_ms": 250
}
```

#### Health Check
```http
GET /health
```

Response:
```json
{
    "status": "healthy",
    "uptime": "2d 15h 30m",
    "storage": {
        "total_size": "1.2GB",
        "free_space": "10.8GB"
    },
    "raft_status": {
        "role": "follower",
        "term": 5,
        "leader_id": "node-1",
        "last_heartbeat": "2024-01-20T15:29:58Z"
    }
}
```

### Error Responses

All endpoints return standard error responses in the following format:

```json
{
    "error": true,
    "code": "INVALID_REQUEST",
    "message": "Invalid metric key format",
    "details": {
        "field": "key",
        "constraint": "must match pattern ^[a-zA-Z0-9_.-]+$"
    }
}
```

Common error codes:
- `INVALID_REQUEST`: Malformed request or invalid parameters
- `NOT_FOUND`: Requested resource not found
- `INTERNAL_ERROR`: Server-side error
- `RAFT_ERROR`: Consensus-related error
- `STORAGE_ERROR`: Database or storage-related error

## Error Handling

#### Validation Errors
```json
{
    "error": true,
    "code": "VALIDATION_ERROR",
    "message": "Invalid request parameters",
    "details": {
        "fields": [
            {
                "field": "from",
                "error": "must be a valid ISO 8601 timestamp"
            },
            {
                "field": "window",
                "error": "must be one of: 1m, 5m, 1h, 1d"
            }
        ]
    }
}
```

#### Authentication Errors
```json
{
    "error": true,
    "code": "AUTH_ERROR",
    "message": "Invalid or expired authentication token",
    "details": {
        "token_status": "expired",
        "expires_at": "2024-01-20T00:00:00Z"
    }
}
```

#### Rate Limit Errors
```json
{
    "error": true,
    "code": "RATE_LIMIT_EXCEEDED",
    "message": "Too many requests",
    "details": {
        "limit": 1000,
        "window": "1m",
        "reset_at": "2024-01-20T15:31:00Z"
    }
}
```

#### Raft Consensus Errors
```json
{
    "error": true,
    "code": "RAFT_ERROR",
    "message": "Failed to achieve consensus",
    "details": {
        "term": 5,
        "required_nodes": 3,
        "available_nodes": 2,
        "retry_after": "5s"
    }
}
```

#### Storage Errors
```json
{
    "error": true,
    "code": "STORAGE_ERROR",
    "message": "Failed to write data",
    "details": {
        "storage_type": "duckdb",
        "operation": "insert",
        "reason": "disk_full",
        "available_space": "100MB"
    }
}
```

## Testing with Postman

### 1. Insert Data (via Control Node)
```
Method: POST
URL: http://localhost:8080/insert
Headers: 
  Content-Type: application/json
  Authorization: Bearer your-auth-token
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
  Authorization: Bearer your-auth-token
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
  Authorization: Bearer your-auth-token
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
  Authorization: Bearer your-auth-token
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