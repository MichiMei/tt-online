package controller.activities;

import gui.MainFrame;
import gui.activities.FastReadPanel;
import org.json.JSONObject;

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
    public void displayPressed(String token, int duration) {
        JSONObject json = new JSONObject();
        json.put("token", token);
        json.put("duration", duration);
        String message = json.toString();
        callbacks.sendUpdate(message);
    }

    @Override
    public void quitPressed() {
        callbacks.endActivity();
    }
}
