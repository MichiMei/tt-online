Online WebApp for the TherapistsToolbox

It will consist of 3 modules:

1. TT_Backend will be a WebSocket Server (written in Rust) which allows for communication between Host and Clients
2. tt-web-app will be a React WebApp for Clients to connect to the Host. It will connect to the Backend via WebSocket to get updates and send user inputs
3. TT-Host(?) will be either a Java Application or a React WebApp for the Host. It will either connect to the Backend via TCP or WebSocket to send updates to the Clients and receive (client) user inputs
