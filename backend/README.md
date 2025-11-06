# WASM-as-OS Backend

## Setup
```bash
cd backend
go mod tidy
go run main.go
```

## API Endpoints
- `GET /api/tasks` - List running tasks
- `POST /api/tasks` - Upload WASM file and start task
- `DELETE /api/tasks/:id` - Stop task

Server runs on http://localhost:8080