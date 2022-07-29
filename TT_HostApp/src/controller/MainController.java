package controller;

import gui.MainFrame;
import transport.Connection;

import java.io.IOException;

public class MainController implements MainFrame.GuiControllerCallbacks, Connection.ConnectionControllerCallbacks {

    private final MainFrame gui;
    private final Connection connection;

    private static final int ACTIVITY_COUNT = 1;

    public MainController(String ip, int port) {
        gui = new MainFrame(this, ACTIVITY_COUNT);
        try {
            connection = new Connection(this, ip, port);
        } catch (IOException e) {
            // TODO handle Error
            // TODO warn user
            // TODO try reconnect
            throw new RuntimeException(e);
        }
    }

    @Override
    public void messageReceived(String json) {
        // TODO handle message
    }

    @Override
    public void activityStarted(int index) {
        // TODO handle activity started

        // create activityController
            // creates activityGui
    }

    public interface ActivityController {
        void inputReceived(String input);
    }
}
