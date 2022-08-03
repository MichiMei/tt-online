package controller.activities;

import gui.MainFrame;

import java.util.ResourceBundle;

public class ActivityControllerFactory {

    public static class BadActivityIndexException extends Exception {
        public BadActivityIndexException(String msg) {
            super(msg);
        }
    }

    public static ActivityController createActivityController(MainFrame mainGui, int index) throws BadActivityIndexException {
        return switch (index) {
            case 0 -> new FastReadController(mainGui);
            default -> throw new BadActivityIndexException("Index " + index + " is not implemented");
        };
    }

    public static String getActivityName(int index) throws BadActivityIndexException {
        return switch (index) {
            case 0 -> "ActivityFastRead";
            default -> throw new BadActivityIndexException("Index " + index + " is not implemented");
        };
    }

    public static String getActivityEndedName() {
        return "None";
    }

    public static String getPrettyActivityName(int index) throws BadActivityIndexException {
        String name = getActivityName(index);
        ResourceBundle strLiterals = ResourceBundle.getBundle("StringLiterals");
        return strLiterals.getString(name);
    }

    public static int getActivityCount() {
        return 1;
    }
}
