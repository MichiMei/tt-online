package gui;

import javax.swing.*;
import javax.swing.plaf.basic.BasicSplitPaneUI;
import java.awt.*;
import java.awt.event.MouseEvent;
import java.awt.event.MouseListener;
import java.util.ResourceBundle;

public class MainFrame extends JFrame {

    public interface GuiControllerCallbacks {
        void activityStarted(int index);
    }

    public static void main(String[] args) {
        /*MainFrame mainFrame = */new MainFrame(null, 1);
    }

    private JPanel panelMain;
    private JList<String> listConnectedUsers;
    private JPanel panelContent;
    private JSplitPane splitPane;
    private final DefaultListModel<String> connectedUsers;

    private final ResourceBundle strLiterals = ResourceBundle.getBundle("Resources/StringLiterals");
    private final GuiControllerCallbacks cb;

    /**
     * Creates a new MainFrame for the GUI
     * @param cb callback functions for user input
     */
    public MainFrame(GuiControllerCallbacks cb, int activityCount) {
        super("TT Host");

        this.cb = cb;

        initializeWindow();
        initializeMenuBar();

        setContentPane(panelMain);

        // INNER CONTENT
        panelContent.add(new ActivitySelection(cb, activityCount), BorderLayout.CENTER);

        // CONNECTED USERS LIST
        connectedUsers = new DefaultListModel<>();  // replace with SortedListModel: https://www.oracle.com/technical-resources/articles/javase/sorted-jlist.html
        listConnectedUsers.setModel(connectedUsers);

        this.setVisible(true);

        // SPLIT PANE
        splitPane.setEnabled(false);
        splitPane.setDividerSize(15);
        splitPane.setDividerLocation(splitPane.getWidth()-splitPane.getRightComponent().getMinimumSize().width-splitPane.getDividerSize()-1);
        ((BasicSplitPaneUI)splitPane.getUI()).getDivider().addMouseListener(new DividerClickListener(splitPane));
    }

    private void initializeWindow() {
        setDefaultCloseOperation(JFrame.EXIT_ON_CLOSE);
        try {
            UIManager.setLookAndFeel(UIManager.getSystemLookAndFeelClassName());
        } catch (ClassNotFoundException | InstantiationException | IllegalAccessException
                 | UnsupportedLookAndFeelException e1) {
            System.err.println("Setting 'LookAndFeel' to native style failed");
            e1.printStackTrace();
        }
        setMinimumSize(new Dimension(1000,750));
    }

    private void initializeMenuBar() {
        // MENU bar //
        JMenuBar menuBar = new JMenuBar();
        setJMenuBar(menuBar);

        JMenu menuData = new JMenu(strLiterals.getString("File"));
        menuBar.add(menuData);
        JMenuItem menuItemSettings = new JMenuItem(strLiterals.getString("Settings"));
        menuData.add(menuItemSettings);
        //menuItemSettings.addActionListener(..);
        JMenuItem menuItemExit = new JMenuItem(strLiterals.getString("Exit_App"));
        menuData.add(menuItemExit);
        menuItemExit.addActionListener(e -> {
            setVisible(false);
            dispose();
        });

        JMenu menuHelp = new JMenu(strLiterals.getString("Help"));
        menuBar.add(menuHelp);
        JMenuItem menuItemAbout = new JMenuItem(strLiterals.getString("About_App"));
        menuHelp.add(menuItemAbout);
        //menuItemAbout.addActionListener(..);
    }

    /**
     * Adds a user to the list of connected users
     * @param user String representation of the user
     */
    public void userConnected(String user) {
        SwingUtilities.invokeLater(() -> {
            connectedUsers.addElement(user);
            assert (connectedUsers.lastElement().equals(user));
        });
    }

    /**
     * Removes the given user from the List
     * @param user String representation of the user to be removed
     */
    public void userDisconnected(String user) {
        SwingUtilities.invokeLater(() -> {
            int index = connectedUsers.indexOf(user);
            if (index < 0) {
                System.err.println("userDisconnected(" + user + "): could not find the user. This should not happen!");
                return;
            }
            connectedUsers.remove(index);
        });
    }

    static class DividerClickListener implements MouseListener {

        private final JSplitPane splitPane;

        public DividerClickListener(JSplitPane splitPane) {
            this.splitPane = splitPane;
        }

        private int getOpenPosition() {
            return splitPane.getWidth()-splitPane.getRightComponent().getMinimumSize().width-splitPane.getDividerSize()-1;
        }

        private int getClosedPosition() {
            return splitPane.getWidth()-splitPane.getDividerSize()-1;
        }

        @Override
        public void mouseClicked(MouseEvent e) {
            if (splitPane.getDividerLocation() >= getClosedPosition()) {
                splitPane.setDividerLocation(getOpenPosition());
            } else {
                splitPane.setDividerLocation(getClosedPosition());
            }
        }

        @Override
        public void mousePressed(MouseEvent e) {}
        @Override
        public void mouseReleased(MouseEvent e) {}
        @Override
        public void mouseEntered(MouseEvent e) {}
        @Override
        public void mouseExited(MouseEvent e) {}
    }
}
