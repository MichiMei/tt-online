package controller;

import controller.activities.ActivityController;
import controller.activities.ActivityControllerFactory;
import controller.activities.ControllerCallbacks;
import gui.MainFrame;
import transport.MessageLayer;

import java.io.IOException;
import java.util.Random;

public class MainController implements MainFrame.GuiControllerCallbacks, MessageLayer.MessageLayerToControllerCallbacks, ControllerCallbacks {

    private final MainFrame gui;
    private final MessageLayer messageLayer;
    private final Random random = new Random();
    private ActivityController activityController = null;
    private int stateId;

    public MainController(String ip, int port) {
        gui = new MainFrame(this, ActivityControllerFactory.getActivityCount());
        try {
            messageLayer = new MessageLayer(this, ip, port);
        } catch (IOException e) {
            // TODO handle Error
            // TODO warn user
            // TODO try reconnect
            throw new RuntimeException(e);
        }
    }

    @Override
    public void handleClientConnected(String name, String address) {
        gui.userConnected(name);
    }

    @Override
    public void handleClientDisconnected(String name, String address, String reason) {
        gui.userDisconnected(name);
    }

    @Override
    public void handleBackendDisconnect(String reason) {
        // TODO notify user
        // TODO Ask if reconnect should be tried
    }

    @Override
    public void handleClientInput(int stateId, String name, String address, String input) {
        if (activityController != null && stateId == this.stateId) {
            activityController.inputReceived(address, name, input);
        }
    }

    @Override
    public void handleOwnDisconnect(String reason) {
        // TODO notify user
        // TODO Ask if reconnect should be tried
    }

    @Override
    public void activityStarted(int index) {
        try {
            activityController = ActivityControllerFactory.createActivityController(gui, this, index);
            stateId = random.nextInt();
            String activityName = ActivityControllerFactory.getActivityName(index);
            messageLayer.sendStateChange(stateId, activityName);
        } catch (ActivityControllerFactory.BadActivityIndexException e) {
            System.err.println("BadActivityIndex");
            activityEnded();
        } catch (MessageLayer.SendingFailedException e) {
            // TODO notify user
            // TODO Ask if reconnect should be tried
        }

    }

    @Override
    public void activityEnded() {
        System.out.println("MainController::activityEnded()");
        activityController = null;
        gui.setActivitySelection();
        String activityName = ActivityControllerFactory.getActivityEndedName();
        try {
            messageLayer.sendStateChange(0, activityName);
        } catch (MessageLayer.SendingFailedException e) {
            // TODO notify user
            // TODO Ask if reconnect should be tried
        }
    }

    @Override
    public void sendUpdate(String update) {
        try {
            messageLayer.sendUpdate(stateId, update);
        } catch (MessageLayer.SendingFailedException e) {
            // TODO handle exception
        }
    }

    @Override
    public void endActivity() {
        activityEnded();
    }
}
