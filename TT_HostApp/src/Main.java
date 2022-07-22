import transport.Connection;

public class Main {

    public static void main(String[] args) {
        //new MainFrame();
        Connection connection = new Connection("127.0.0.1", 8081);

        while (true) {} // mock GUI running
    }

}