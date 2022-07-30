package controller;

import controller.activities.ActivityController;
import controller.activities.ActivityControllerFactory;
import gui.MainFrame;
import transport.Connection;

import java.io.IOException;

public class MainController implements MainFrame.GuiControllerCallbacks, Connection.ConnectionControllerCallbacks {

    private final MainFrame gui;
    private final Connection connection;
    private ActivityController activityController = null;

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
        // TODO parse json
        String name = "name_mock";
        String address = "address_mock";
        String content = "content_mock";
        if (activityController != null) {
            activityController.inputReceived(address, name, content);
        }
    }

    @Override
    public void activityStarted(int index) {
        try {
            activityController = ActivityControllerFactory.createActivityController(gui, index);
        } catch (ActivityControllerFactory.BadActivityIndexException e) {
            System.err.println("BadActivityIndex");
            activityEnded();
        }
    }

    @Override
    public void activityEnded() {
        System.out.println("MainController::activityEnded()");
        activityController = null;
        gui.setActivitySelection();
    }
}
