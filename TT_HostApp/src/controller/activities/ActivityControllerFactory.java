package controller.activities;

import gui.MainFrame;

public class ActivityControllerFactory {

    public static class BadActivityIndexException extends Exception {
        public BadActivityIndexException(String msg) {
            super(msg);
        }

        public BadActivityIndexException() {
            super();
        }
    }

    public static ActivityController createActivityController(MainFrame mainGui, int index) throws BadActivityIndexException {
        switch (index) {
            case 0:
                return new FastReadController(mainGui);
            default:
                throw new BadActivityIndexException("Index " + index + " is not implemented");
        }
    }
}
