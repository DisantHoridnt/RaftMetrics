# RaftMetrics: Distributed Analytics System

## Overview
A distributed analytics system implemented in Rust that demonstrates key distributed systems concepts including fault tolerance, data partitioning, and distributed computation. The system uses consistent hashing for data partitioning and efficient metric distribution across worker nodes.

## Architecture
- **Control Node**: Handles incoming requests and routes metrics to appropriate worker nodes using consistent hashing
- **Worker Nodes**: Process and store metrics data, providing both individual and aggregated metrics
- **Partitioning**: Uses Jump Consistent Hashing for even distribution of metrics across workers
- **Metrics**: Utilizes HistogramVec for accurate metric aggregation and statistics

## Features
- Consistent hashing ensures same metrics always route to same worker
- Efficient metric aggregation with proper statistical distribution
- Scalable architecture supporting multiple worker nodes
- Real-time metric processing and aggregation
- Comprehensive logging and debugging capabilities

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

#### 2. Record Metric
```http
POST /metrics
Content-Type: application/json

{
    "metric_name": "cpu_usage",
    "value": 75.5
}

# Response
{
    "success": true,
    "message": "Metric recorded on worker X"
}
```

#### 3. Get Metric
```http
GET /metrics/{name}

# Response
{
    "metric_name": "cpu_usage",
    "values": [75.5, 80.2, 70.1],
    "timestamp": "2024-11-25T20:59:23.376Z"
}
```

#### 4. Get Metric Aggregate
```http
GET /metrics/{name}/aggregate

# Response
{
    "metric_name": "cpu_usage",
    "count": 3,
    "sum": 225.8,
    "mean": 75.27,
    "min": 70.1,
    "max": 80.2,
    "timestamp": "2024-11-25T20:59:23.376Z"
}
```

## Development

### Project Structure
```
src/
├── api/           # API handlers for control and worker nodes
├── metrics/       # Metrics processing and aggregation logic
├── partitioning/  # Consistent hashing implementation
├── proto/         # Protocol buffer definitions
└── raft/          # Consensus implementation
```

### Key Components
1. **Control Node**
   - Routes incoming metrics using consistent hashing
   - Manages worker node coordination
   - Handles API requests and responses

2. **Worker Nodes**
   - Process and store assigned metrics
   - Provide individual and aggregated metric data
   - Maintain metric history and statistics

3. **Partitioning**
   - Implements Jump Consistent Hashing
   - Ensures even distribution of metrics
   - Maintains consistency in routing

### Building and Testing
```bash
# Build the project
cargo build

# Run tests
cargo test

# Run with debug logging
RUST_LOG=debug cargo run
```
## License
This project is licensed under the MIT License - see the LICENSE file for details.

## Data Flow Architecture Diagram
![RaftMetrics Data Flow Architecture](https://drive.google.com/uc?export=view&id=1maj3XhDEhCn9RJP9S5ftqMoCogfjuBj4)