# From The Hart Storage

A repository for the storage domain

## Features

- **RESTful API** for file storage and management
- **Health monitoring** endpoint for service status checks
- **Graceful shutdown** handling for production reliability
- **OpenAPI/Swagger** documentation
- **AWS Lambda** support for serverless deployments

## Graceful Shutdown

The application implements graceful shutdown to ensure clean resource management in production environments. When a shutdown signal is received (SIGINT/Ctrl+C or SIGTERM), the server will:

1. Stop accepting new connections
2. Wait for all inflight requests to complete
3. Clean up resources and connections
4. Terminate gracefully

This prevents data loss or corruption during deployments, rolling updates, or manual restarts.

### Signal Handling

The application listens for the following signals:
- **SIGINT** (Ctrl+C): Typically used during development
- **SIGTERM**: Standard termination signal used by container orchestrators (Kubernetes, Docker, etc.)

### Example

```bash
# Start the server
RUST_LOG=info APP_ENVIRONMENT=production APP_SERVER_HOST=0.0.0.0 APP_SERVER_PORT=8080 cargo run

# In another terminal, send a shutdown signal
kill -TERM <pid>

# Or use Ctrl+C in the same terminal
```

The server will log the shutdown initiation and completion:
```
WARN: Received SIGTERM, initiating graceful shutdown
INFO: Server shutdown complete
```
