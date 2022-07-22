package gui;

import javax.swing.*;
import java.awt.*;

public class MainFrame extends JFrame {

    public MainFrame() {
        super("TT Host");
        //JFrame window = this;

        // WINDOW Settings //
        setDefaultCloseOperation(JFrame.EXIT_ON_CLOSE);
        try {
            UIManager.setLookAndFeel(UIManager.getSystemLookAndFeelClassName());
        } catch (ClassNotFoundException | InstantiationException | IllegalAccessException
                 | UnsupportedLookAndFeelException e1) {
            System.err.println("Setting 'LookAndFeel' to native style failed");
            e1.printStackTrace();
        }
        setMinimumSize(new Dimension(1000,750));

        // MENU bar //
        JMenuBar menuBar = new JMenuBar();
        setJMenuBar(menuBar);

        JMenu menuData = new JMenu("Datei");
        menuBar.add(menuData);
        JMenuItem menuItemSettings = new JMenuItem("Einstellungen");
        menuData.add(menuItemSettings);
        //menuItemSettings.addActionListener(..);
        JMenuItem menuItemExit = new JMenuItem("TT Host App Beenden");
        menuData.add(menuItemExit);
        menuItemExit.addActionListener(e -> {
            setVisible(false);
            dispose();
        });

        JMenu menuHelp = new JMenu("Hilfe");
        menuBar.add(menuHelp);
        JMenuItem menuItemAbout = new JMenuItem("Über TT Host App");
        menuHelp.add(menuItemAbout);
        //menuItemAbout.addActionListener(..);

        // CONTENT //
        // TODO create pane
        //setContentPane(pane);
        System.out.println(getContentPane());

        this.setVisible(true);
    }

}
