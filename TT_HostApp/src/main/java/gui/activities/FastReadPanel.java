package gui.activities;

import javax.swing.*;
import javax.swing.event.DocumentEvent;
import javax.swing.event.DocumentListener;
import java.awt.*;

public class FastReadPanel extends JPanel {
    public interface ControllerCallbacks {
        void displayPressed(String token);
    }

    private JPanel mainPanel;
    private JTextArea textArea;
    private JComboBox separatorSelection;
    private JButton quitButton;
    private JButton previousButton;
    private JButton displayButton;
    private JButton nextButton;
    private JLabel statusLabel;
    private JPanel timeSelectionPanel;

    private final ControllerCallbacks callbacks;

    public FastReadPanel(ControllerCallbacks callbacks) {
        super();
        setLayout(new BorderLayout());
        add(mainPanel, BorderLayout.CENTER);
        this.callbacks = callbacks;

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

        displayButton.addActionListener(e -> displayPressed());

        this.setVisible(true);
    }

    private void displayPressed() {
        String token = textArea.getText();
        callbacks.displayPressed(token);
    }

}
