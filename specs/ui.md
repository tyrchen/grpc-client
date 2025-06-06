# UI

## Requirements

1. Allow loading a list of grpc servers through configuration
2. UI will automatically load and show all the details on services / methods / input / output supported when a given server is clicked.
3. user could fill a form (which is generated against the input schema), and send request to server, then render the response in json format (in future, probably table format)

## Design

1. web server should use axum 0.8. With Yaml config and AppState which contains a DashMap of servers.
2. axum APIs shall leverage as much as possible what has been implemented in `src/client.rs`.
3. axum server should be started via CLI. e.g. `grpc-client server`.
4. UI should be built with react, typescript, shadcn/ui and tailwindcss, I already have a skeleton project in `ui` folder.
