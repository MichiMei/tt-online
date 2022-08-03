package gui.activities;

import javax.swing.*;
import javax.swing.event.DocumentEvent;
import javax.swing.event.DocumentListener;
import java.awt.*;

public class FastReadPanel extends JPanel {
    private JPanel mainPanel;
    private JTextArea textArea;
    private JComboBox separatorSelection;
    private JButton quitButton;
    private JButton previousButton;
    private JButton displayButton;
    private JButton nextButton;
    private JLabel statusLabel;
    private JPanel timeSelectionPanel;

    public FastReadPanel() {
        super();
        setLayout(new BorderLayout());
        add(mainPanel, BorderLayout.CENTER);

        textArea.getDocument().addDocumentListener(new DocumentListener() {
            @Override
            public void insertUpdate(DocumentEvent e) {

            }

            @Override
            public void removeUpdate(DocumentEvent e) {

            }

            @Override
            public void changedUpdate(DocumentEvent e) {

            }
        });

        this.setVisible(true);
    }

}
