package transport;

import java.io.*;
import java.net.Socket;
import java.nio.charset.StandardCharsets;

public class ConnectionLayer {

    private final DataOutputStream out;
    private final DataInputStream in;

    public ConnectionLayer(String ip, int port) throws IOException {
        Socket socket = new Socket(ip, port);

        System.out.println("Connected to Host: " + socket.getLocalAddress() + ":" + socket.getLocalPort());
        out = new DataOutputStream(socket.getOutputStream());
        in = new DataInputStream(socket.getInputStream());
    }

    /**
     * Write the given String representation of a message into the socket
     * The message is UTF-8 encoded for compatibility with RUST
     * @param message string representation of a message
     * @throws IOException thrown if socket fails (connection should get closed)
     */
    public synchronized void sendMessage(String message) throws IOException {
        System.out.println("Sending message " + message);
        byte[] utf8 = message.getBytes(StandardCharsets.UTF_8);
        int length = utf8.length;
        System.out.println("length " + length);

        out.writeInt(length);
        out.write(utf8);
        out.flush();
    }

    /**
     * Receive the string representation of the next message
     * @return string representation of the next message
     * @throws IOException thrown if socket fails or connection was closed (connection should get closed)
     */
    public String receiveMessage() throws IOException {
        int length = in.readInt();

        byte[] utf8 = in.readNBytes(length);

        String str = new String(utf8, StandardCharsets.UTF_8);
        System.out.println("Message decoded: " + str);

        return str;
    }

    /**
     * Will close sending and receiving half of the socket
     * No further send and receive operations should be called
     * @throws IOException thrown if one of the socket halves could not be closed
     */
    public void close() throws IOException {
        in.close();
        out.close();
    }

    /**
     * Will close the sending half of the socket
     * No further send operations should be called
     * @throws IOException thrown if one of the socket half could not be closed
     */
    public void closeSender() throws IOException {
        in.close();
    }
}
