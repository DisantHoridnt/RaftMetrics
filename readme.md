# RaftMetrics: Distributed Analytics System

## Overview
A distributed analytics system implemented in Rust that demonstrates key distributed systems concepts including fault tolerance, data partitioning, and distributed computation. The system uses Raft consensus for reliable data replication across nodes.

## Architecture
- **Control Node**: Handles incoming requests and coordinates worker nodes
- **Worker Nodes**: Process and store metrics data using DuckDB
- **Consensus**: Uses Raft protocol for data replication and node coordination

## Quick Start with Docker Compose

### Prerequisites
- Docker
- Docker Compose
- Postman (for testing)

### Running the Stack
1. Clone the repository
2. Navigate to the project directory
3. Start the stack:
```bash
docker-compose up -d
```

This will start:
- Control Node: http://localhost:8080
- Worker Node 1: http://localhost:8081
- Worker Node 2: http://localhost:8082

## API Reference

### Authentication
All requests require the following header:
```http
Authorization: Bearer local-dev-token-123
```

### Endpoints

#### 1. Health Check
```http
GET /health

# Response
{
    "status": "healthy",
    "message": "[node] is operational"
}
```

#### 2. Metrics Management

##### Record Metric
```http
POST /metrics
Content-Type: application/json

{
    "metric_name": "cpu_usage",
    "value": 75.5,
    "timestamp": "2024-01-20T15:30:00Z",
}
```

##### Get Metric
```http
GET /metrics/:name

# Response
{
    "metric_name": "cpu_usage",
    "value": 75.5
}
```

##### Aggregate Metrics
```http
GET /aggregate?metric=cpu_usage&from=2024-01-20T00:00:00Z&to=2024-01-20T23:59:59Z

# Response
{
    "metric": "cpu_usage",
    "aggregations": {
        "avg": 68.5,
        "min": 45.2,
        "max": 92.1,
        "count": 1440
    }
}
```

### Query Parameters
- `from`: Start timestamp (ISO 8601)
- `to`: End timestamp (ISO 8601)
- `window`: Time window for aggregation (e.g., "1h", "1d")
- `agg`: Aggregation functions (avg, min, max, sum, count)

## Monitoring and Troubleshooting

### Docker Commands
- View all logs: `docker-compose logs -f`
- View specific service: `docker-compose logs -f control`
- Restart services: `docker-compose restart`
- Stop stack: `docker-compose down`

### Common Issues
1. **Service Unavailable**: Check if all containers are running with `docker-compose ps`
2. **Authentication Failed**: Verify the auth token in the request header
3. **Connection Refused**: Ensure correct port mapping in docker-compose.yml

## Technology Stack
- **Language**: Rust
- **Web Framework**: Axum
- **Database**: DuckDB
- **Consensus**: Raft (tikv/raft-rs)
- **Runtime**: Tokio
- **Serialization**: Serde
- **Logging**: Tracing

## Development

### Building from Source
```bash
cargo build --release
```

### Running Tests
```bash
cargo test
```

### Configuration
Key environment variables:
- `NODE_ROLE`: "control" or "worker"
- `NODE_ID`: Unique node identifier
- `PORT`: Service port
- `AUTH_TOKEN`: Authentication token
- `WORKER_HOSTS`: Comma-separated worker hosts
