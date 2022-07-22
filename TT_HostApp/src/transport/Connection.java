package transport;

import java.io.*;
import java.net.Socket;
import java.nio.charset.StandardCharsets;

public class Connection {

    //private ObjectOutputStream out;
    private final DataOutputStream out;

    public Connection(String ip, int port) {
        try {
            Socket socket = new Socket(ip, port);

            System.out.println("Connected to Host: " + socket.getLocalAddress() + ":" + socket.getLocalPort());
            out = new DataOutputStream(socket.getOutputStream());
            DataInputStream in = new DataInputStream(socket.getInputStream());

            Thread reader = new Thread(new Receiver(in));
            reader.start();
            System.out.println("Reader Thread started");
        } catch (IOException e) {
            // TODO
            throw new RuntimeException(e);
        }
    }


    public void writeJson(String json) throws IOException {
        System.out.println("Sending Json " + json);
        byte[] utf8 = json.getBytes(StandardCharsets.UTF_8);
        int length = utf8.length;
        System.out.println("length " + length);

        out.writeInt(length);
        out.write(utf8);
        out.flush();
    }

    public void closeConnection() throws IOException {
        out.close();
    }

    static class Receiver implements Runnable {

        private final DataInputStream in;

        public Receiver(DataInputStream in) {
            this.in = in;
        }

        @Override
        public void run() {
            while (true) {
                int length;
                try {
                    length = in.readInt();
                } catch (EOFException e) {
                    // TODO connection closed
                    System.out.println("Received EOF");
                    return ;
                } catch (IOException e) {
                    // TODO handle exception
                    System.out.println("IOException: " + e.getMessage());
                    continue;
                }
                byte[] utf8;
                try {
                    utf8 = in.readNBytes(length);
                } catch (IOException e) {
                    // TODO handle exception
                    continue;
                }
                String json = new String(utf8, StandardCharsets.UTF_8);
                System.out.println("Message decoded: " + json);
                // TODO callback json
            }
        }

    }

}
