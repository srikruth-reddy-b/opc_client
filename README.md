# Object Oriented Programming in Rust

# OPC Client
This project implements an **OPC Client** that interacts with an OPC server and a WebSocket server. The client performs the following tasks:

1. **Fetches tags** from a WebSocket server.
2. **Matches tags** with an OPC server.
3. **Subscribes** to the OPC server tags and listens for changes.
4. **Pushes changed values** back to the WebSocket server.


## Features

- Connects to an OPC server and retrieves tag values.
- Subscribes to OPC tags for real-time data updates.
- Pushes updated values to a WebSocket server as they change.
- Supports secure WebSocket (wss) and regular WebSocket (ws) connections.
- Provides configuration options for OPC server connection parameters and WebSocket server address.

## Technologies Used

- **Rust**: The client is written in Rust, using asynchronous programming for efficient real-time communication.
- **OPC UA **: The client communicates with an OPC UA server using the `opcua` crate.
- **WebSocket**: Real-time communication with the WebSocket server is achieved through the `tokio-tungstenite` crate.

## Prerequisites

Before running the client, ensure that you have the following installed:

- **Rust**: Install Rust via [rustup](https://rustup.rs/).
- **OPC UA Server**: You will need access to an OPC server that exposes tags for monitoring.
- **WebSocket Server**: A WebSocket server should be running that can accept and handle tag data.

## Setup

### Clone the Repository

```bash
git clone https://github.com/yourusername/opc-client.git
cd opc-client
