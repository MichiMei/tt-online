package transport;

import org.json.JSONException;
import org.json.JSONObject;

import java.io.IOException;

public class MessageLayer  {

    public static final String DISCONNECT_REASON_VIOLATION = "Protocol violation";
    public static final String DISCONNECT_REASON_BACKEND_CLOSED_GRACEFULLY = "Connection closed gracefully by backend";

    public interface MessageLayerToControllerCallbacks {
        // new callbacks
        void handleClientConnected(String name, String address);
        void handleClientDisconnected(String name, String address, String reason);
        void handleBackendDisconnect(String reason);
        void handleClientInput(int stateId, String name, String address, String input);
        void handleOwnDisconnect(String reason);
    }

    private final MessageLayerToControllerCallbacks callbacks;
    private final ConnectionLayer connectionLayer;

    public MessageLayer(MessageLayerToControllerCallbacks callbacks, String ip, int port) throws IOException {
        this.callbacks = callbacks;
        this.connectionLayer = new ConnectionLayer(ip, port);

        Thread messageReceiver = new Thread(new MessageReceiver());
        messageReceiver.start();
    }

    /**
     * Sends an update message to the backend
     * @param update content of the update message
     * @throws SendingFailedException thrown if sending fails and socket was closed
     */
    public void sendUpdate(int stateId, String update) throws SendingFailedException {
        JSONObject json = new JSONObject();
        json.put("type", "Update");
        json.put("state_id", stateId);
        json.put("content", update);
        String message = json.toString();

        try {
            connectionLayer.sendMessage(message);
        } catch (IOException e) {
            forceClose();
            throw new SendingFailedException();
        }
    }

    /**
     * Sends a state change message to the backend
     * @param state content of the state change message
     * @throws SendingFailedException thrown if sending fails and socket was closed
     */
    public void sendStateChange(int stateId, String state) throws SendingFailedException {
        JSONObject json = new JSONObject();
        json.put("type", "ChangeState");
        json.put("state_id", stateId);
        json.put("content", state);
        String message = json.toString();

        try {
            connectionLayer.sendMessage(message);
        } catch (IOException e) {
            forceClose();
            throw new SendingFailedException();
        }
    }

    /**
     * Sends a disconnect message to the backend and closes the sending socket half
     * @param reason reason for the disconnect
     * @throws SendingFailedException thrown if sending fails and socket was closed completely
     */
    public void closeSend(String reason) throws SendingFailedException {
        JSONObject json = new JSONObject();
        json.put("type", "Disconnecting");
        json.put("reason", reason);
        String message = json.toString();

        try {
            connectionLayer.sendMessage(message);
        } catch (IOException e) {
            forceClose();
            throw new SendingFailedException();
        }

        try {
            connectionLayer.closeSender();
        } catch (IOException e) {
            // TODO log
        }
    }

    public void closeCompletely(String reason) {
        JSONObject json = new JSONObject();
        json.put("type", "Disconnecting");
        json.put("reason", reason);
        String message = json.toString();

        try {
            connectionLayer.sendMessage(message);
        } catch (IOException e) {
            // TODO log
        }

        forceClose();
        callbacks.handleOwnDisconnect(reason);
    }

    private void forceClose() {
        try {
            connectionLayer.close();
        } catch (IOException e) {
            // TODO log
        }
    }

    class MessageReceiver implements Runnable {
        public MessageReceiver() {

        }

        @Override
        public void run() {
            while (true) {
                String message;
                try {
                    message = connectionLayer.receiveMessage();
                } catch (IOException e) {
                    handleProtocolViolation();
                    break;
                }

                try {
                    if (!parseMessage(message)) {
                        closeCompletely(DISCONNECT_REASON_BACKEND_CLOSED_GRACEFULLY);
                    }
                } catch (JSONParseException e) {
                    handleProtocolViolation();
                    break;
                }

            }
        }

        private void handleProtocolViolation() {
            closeCompletely(DISCONNECT_REASON_VIOLATION);
        }

        private boolean parseMessage(String str) throws JSONParseException {
            try {
                JSONObject json = new JSONObject(str);
                String type = json.getString("type");
                switch (type) {
                    case "ClientConnected" -> parseClientConnected(json);
                    case "ClientDisconnected" -> parseClientDisconnected(json);
                    case "Disconnect" -> {
                        parseDisconnect(json);
                        return false;
                    }
                    case "Input" -> parseInput(json);
                    default -> throw new JSONParseException("Type is not supported: " + type);
                }

            } catch (JSONException e) {
                throw new JSONParseException("JSON is malformed: " + str);
            }
            return true;
        }

        private void parseClientConnected(JSONObject json) throws JSONParseException {
            try {
                String name = json.getString("name");
                String address = json.getString("address");
                callbacks.handleClientConnected(name, address);
            } catch (JSONException e) {
                throw new JSONParseException("ClientConnected message is malformed: " + json);
            }
        }

        private void parseClientDisconnected(JSONObject json) throws JSONParseException {
            try {
                String name = json.getString("name");
                String address = json.getString("address");
                String reason = json.getString("reason");
                callbacks.handleClientDisconnected(name, address, reason);
            } catch (JSONException e) {
                throw new JSONParseException("ClientDisconnected message is malformed: " + json);
            }
        }

        private void parseDisconnect(JSONObject json) throws JSONParseException {
            try {
                String reason = json.getString("reason");
                callbacks.handleBackendDisconnect(reason);
            } catch (JSONException e) {
                throw new JSONParseException("Disconnect message is malformed: " + json);
            }
        }

        private void parseInput(JSONObject json) throws JSONParseException {
            try {
                int stateId = json.getInt("state_id");
                String name = json.getString("name");
                String address = json.getString("address");
                String input = json.getString("content");
                callbacks.handleClientInput(stateId, name, address, input);
            } catch (JSONException e) {
                throw new JSONParseException("Input message is malformed: " + json);
            }
        }
    }

    public static class SendingFailedException extends Exception {
        public SendingFailedException() {
            super();
        }
    }

    static class JSONParseException extends Exception {
        public JSONParseException(String msg) {
            super(msg);
        }
    }

}
