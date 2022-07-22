package transport;

import java.io.*;
import java.net.Socket;
import java.nio.charset.StandardCharsets;

public class Connection {

    //private ObjectOutputStream out;
    private final DataOutputStream out;

    public Connection(String ip, int port) {
        try (Socket socket = new Socket(ip, port)) {
            //out = new ObjectOutputStream(socket.getOutputStream());
            out = new DataOutputStream(socket.getOutputStream());
            DataInputStream in = new DataInputStream(socket.getInputStream());
            Thread reader = new Thread(new Receiver(in));
            reader.start();
        } catch (IOException e) {
            throw new RuntimeException(e);
        }
    }


    public void writeJson(String json) throws IOException {
        byte[] utf8 = json.getBytes(StandardCharsets.UTF_8);
        int length = utf8.length;
        assert(Integer.SIZE == 32);
        out.write(length);
        out.write(utf8);
        out.flush();
        // TODO check if rust parses length and 'json' correctly
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
                    return ;
                } catch (IOException e) {
                    // TODO handle exception
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
                // TODO callback json
            }
        }

    }

}
