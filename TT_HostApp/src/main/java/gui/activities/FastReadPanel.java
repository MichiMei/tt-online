package gui.activities;

import javax.swing.*;
import javax.swing.event.DocumentEvent;
import javax.swing.event.DocumentListener;
import javax.swing.text.BadLocationException;
import javax.swing.text.DefaultHighlighter;
import javax.swing.text.Highlighter;
import java.awt.*;
import java.awt.event.ActionEvent;
import java.util.ArrayList;
import java.util.List;
import java.util.ResourceBundle;
import java.util.function.Function;

public class FastReadPanel extends JPanel {
    public interface ControllerCallbacks {
        void displayPressed(String token);
    }

    private JPanel mainPanel;
    private JTextArea textArea;
    private JComboBox<Tokenizer.Separator> separatorSelection;
    private JButton quitButton;
    private JButton previousButton;
    private JButton displayButton;
    private JButton nextButton;
    private JLabel statusLabel;
    private JPanel timeSelectionPanel;

    private final ControllerCallbacks callbacks;
    private final ResourceBundle strLiterals = ResourceBundle.getBundle("StringLiterals");
    private final Tokenizer tokenizer;

    public FastReadPanel(ControllerCallbacks callbacks) {
        super();
        setLayout(new BorderLayout());
        add(mainPanel, BorderLayout.CENTER);
        this.callbacks = callbacks;

        tokenizer = new Tokenizer(textArea, separatorSelection);

        displayButton.addActionListener(e -> {
            try {
                displayPressed(tokenizer.getSelected());
            } catch (Tokenizer.EmptyTokenSelectedException ex) {
                // TODO handle exception
                throw new RuntimeException(ex);
            }
        });
        nextButton.addActionListener(e -> tokenizer.selectNext());
        previousButton.addActionListener(e -> tokenizer.selectPrev());

        this.setVisible(true);
    }

    private void displayPressed(String token) {
        callbacks.displayPressed(token);
    }

    private class Tokenizer {

        private final JTextArea textArea;
        private final JComboBox<Separator> separatorSelection;

        String text;
        private Separator selectedSeparator;
        private List<Integer> separatorPositions;
        private int selectedToken = 0;

        public Tokenizer(JTextArea textArea, JComboBox<Separator> separatorSelection) {
            this.textArea = textArea;
            this.separatorSelection = separatorSelection;

            addTextAreaListener();
            addSeparatorSelectionItems();
            addSeparatorSelectionListener();

            tokenize();
        }

        private void addTextAreaListener() {
            textArea.getDocument().addDocumentListener(new DocumentListener() {
                @Override
                public void insertUpdate(DocumentEvent e) {
                    tokenize();
                }

                @Override
                public void removeUpdate(DocumentEvent e) {
                    tokenize();
                }

                @Override
                public void changedUpdate(DocumentEvent e) {
                    tokenize();
                }
            });
        }

        private void addSeparatorSelectionItems() {
            addSeparatorSelectionItem("whitespaces", Character::isWhitespace);
            addSeparatorSelectionItem("comma", c -> c.equals(','));
            addSeparatorSelectionItem("semicolon", c -> c.equals(';'));
            selectedSeparator = (Separator) separatorSelection.getSelectedItem();
        }

        private void addSeparatorSelectionItem(String resourceName, Function<Character, Boolean> compareToSeparator) {
            String name = strLiterals.getString(resourceName);
            Separator separator = new Separator(name, compareToSeparator);
            separatorSelection.addItem(separator);
        }

        private void addSeparatorSelectionListener() {
            separatorSelection.addActionListener(new AbstractAction() {
                @Override
                public void actionPerformed(ActionEvent e) {
                    selectedSeparator = (Separator) separatorSelection.getSelectedItem();
                    tokenize();
                }
            });
        }

        private void tokenize() {
            text = textArea.getText();

            separatorPositions = new ArrayList<>();
            separatorPositions.add(-1);                 // implicit start separator
            for (int i = 0; i < text.length(); i++) {
                if (selectedSeparator.compareToSeparator.apply(text.charAt(i))) {
                    separatorPositions.add(i);
                }
            }
            separatorPositions.add(text.length());      // implicit end separator

            resetSelectedToken();
            setHighlight();
        }

        private void resetSelectedToken() {
            if (selectedToken >= separatorPositions.size()-1) {
                // if old selected is now out of bounds -> select default (0)
                selectFirstNonEmptyToken();
                return;
            }
            String selected = getToken(selectedToken);
            if (selected.length() == 0) {
                // if old selected is now empty -> try next
                if (selectNext()) {
                    return;
                }
                // else try prev
                selectPrev();
            }
        }

        private void selectFirstNonEmptyToken() {
            selectedToken = 0;
            if (getToken(selectedToken).length() == 0) {
                selectNext();
            }
        }

        public String getSelected() throws EmptyTokenSelectedException {
            String token = getToken(selectedToken);
            if (token.length() == 0) {
                throw new EmptyTokenSelectedException();
            }
            return token;
        }

        private String getToken(int index) {
            int start = separatorPositions.get(index)+1;
            int end = separatorPositions.get(index+1);
            return text.substring(start, end).trim();
        }

        public boolean selectNext() {
            int newSelected = selectedToken;
            for (int nextIndex = newSelected+1; nextIndex < separatorPositions.size()-1; nextIndex++) {
                String nextToken = getToken(nextIndex);
                if (nextToken.length() > 0) {
                    newSelected = nextIndex;
                    break;
                }
            }
            if (newSelected == selectedToken) {
                return false;
            } else {
                selectedToken = newSelected;
                setHighlight();
                return true;
            }
        }

        public boolean selectPrev() {
            int newSelected = selectedToken;
            for (int prevIndex = newSelected-1; prevIndex >= 0; prevIndex--) {
                String prevToken = getToken(prevIndex);
                if (prevToken.length() > 0) {
                    newSelected = prevIndex;
                    break;
                }
            }
            if (newSelected == selectedToken) {
                return false;
            } else {
                selectedToken = newSelected;
                setHighlight();
                return true;
            }
        }

        private void setHighlight() {
            Highlighter highlighter = textArea.getHighlighter();
            Highlighter.HighlightPainter painter = new DefaultHighlighter.DefaultHighlightPainter(Color.yellow);
            highlighter.removeAllHighlights();
            try {
                int start = separatorPositions.get(selectedToken)+1;
                int end = separatorPositions.get(selectedToken+1);
                highlighter.addHighlight(start, end, painter);
            } catch (BadLocationException ex) {
                ex.printStackTrace();
                // TODO
            }

        }

        private record Separator(String name, Function<Character, Boolean> compareToSeparator) {
            @Override
            public String toString() {
                return name;
            }
        }

        public  static class EmptyTokenSelectedException extends Exception {
            public EmptyTokenSelectedException() {
                super();
            }
        }
    }
}
