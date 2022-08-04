package gui;

import controller.activities.ActivityControllerFactory;

import javax.swing.*;
import java.awt.*;
import java.util.ResourceBundle;

public class ActivitySelection extends JPanel {

    public static void main(String[] args) {
        JFrame window = new JFrame();
        ActivitySelection activitySelection = new ActivitySelection(null, ActivityControllerFactory.getActivityCount());
        window.setContentPane(activitySelection);
        window.setDefaultCloseOperation(WindowConstants.EXIT_ON_CLOSE);
        window.setSize(1000,750);
        window.setVisible(true);
    }

    private JPanel mainPanel;
    private JButton buttonStartActivity;
    private JList<String> listActivities;
    private final ResourceBundle strLiterals = ResourceBundle.getBundle("StringLiterals");
    private final MainFrame.GuiControllerCallbacks cb;

    public ActivitySelection(MainFrame.GuiControllerCallbacks cb, int activityCount) {
        super();
        this.cb = cb;
        setLayout(new BorderLayout());
        add(mainPanel, BorderLayout.CENTER);

        addActivities(activityCount);

        buttonStartActivity.addActionListener(e -> activityStarted());

        this.setVisible(true);
    }

    private void activityStarted() {
        int selected = listActivities.getSelectedIndex();
        if (selected == -1) {
            JOptionPane.showMessageDialog(this, strLiterals.getString("Info_NoActivitySelected_Message"), strLiterals.getString("Info_NoActivitySelected_Title"), JOptionPane.INFORMATION_MESSAGE);
            return;
        }
        cb.activityStarted(selected);
    }

    private void addActivities(int activityCount) {
        System.out.println("Add " + activityCount + " activities");
        DefaultListModel<String> listModel = new DefaultListModel<>();
        listActivities.setModel(listModel);

        for (int i = 0; i < activityCount; i++) {
            try {
                listModel.addElement(ActivityControllerFactory.getPrettyActivityName(i));
            } catch (ActivityControllerFactory.BadActivityIndexException e) {
                throw new RuntimeException(e);
            }
        }
    }

}
