package controller.activities;

import gui.MainFrame;
import gui.activities.FastReadPanel;

public class FastReadController implements ActivityController, FastReadPanel.ControllerCallbacks {
    private final FastReadPanel gui;

    private final ControllerCallbacks callbacks;

    public FastReadController(MainFrame mainGui, ControllerCallbacks callbacks) {
        gui = new FastReadPanel(this);
        mainGui.setActivityGui(gui);
        this.callbacks = callbacks;
    }

    @Override
    public void inputReceived(String address, String name, String input) {
        // TODO
    }

    @Override
    public void displayPressed(String token) {
        callbacks.sendUpdate(token);
    }
}
