package controller.activities;

import gui.MainFrame;
import gui.activities.FastReadPanel;

public class FastReadController implements ActivityController {

    FastReadPanel gui;

    public FastReadController(MainFrame mainGui) {
        gui = new FastReadPanel();
        mainGui.setActivityGui(gui);
    }

    @Override
    public void inputReceived(String address, String name, String input) {
        // TODO
    }
}
