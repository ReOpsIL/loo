use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    terminal::{self, disable_raw_mode, enable_raw_mode, Clear, ClearType},
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
};
use std::io::{stdout, Stdout, Write};
use unicode_width::UnicodeWidthStr;

pub struct TerminalInput {
    exit_count: u8,
    esc_count: u8,
    terminal_size: (u16, u16), // (width, height)
    prompt: String,
}

pub enum InputEvent {
    UserInput(String),
    ExitRequest(u8),
    ClearPrompt,
    Interrupt,
}

struct TextBuffer {
    content: String,
    cursor_pos: usize, // Character position in content (NOT byte position)
    display_offset: usize, // For horizontal scrolling
}

impl TextBuffer {
    fn new() -> Self {
        Self {
            content: String::new(),
            cursor_pos: 0,
            display_offset: 0,
        }
    }

    // Helper function to convert character position to byte position
    fn char_to_byte_pos(&self, char_pos: usize) -> usize {
        self.content
            .char_indices()
            .nth(char_pos)
            .map(|(byte_pos, _)| byte_pos)
            .unwrap_or(self.content.len())
    }

    // Helper function to get character count
    fn char_len(&self) -> usize {
        self.content.chars().count()
    }

    fn insert_char(&mut self, ch: char) {
        let byte_pos = self.char_to_byte_pos(self.cursor_pos);
        self.content.insert(byte_pos, ch);
        self.cursor_pos += 1;
    }

    fn delete_char_before(&mut self) -> bool {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
            let byte_pos = self.char_to_byte_pos(self.cursor_pos);
            self.content.remove(byte_pos);
            true
        } else {
            false
        }
    }

    fn delete_char_at(&mut self) -> bool {
        let char_len = self.char_len();
        if self.cursor_pos < char_len {
            let byte_pos = self.char_to_byte_pos(self.cursor_pos);
            self.content.remove(byte_pos);
            true
        } else {
            false
        }
    }

    fn move_cursor_left(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
        }
    }

    fn move_cursor_right(&mut self) {
        let char_len = self.char_len();
        if self.cursor_pos < char_len {
            self.cursor_pos += 1;
        }
    }

    fn move_cursor_home(&mut self) {
        self.cursor_pos = 0;
    }

    fn move_cursor_end(&mut self) {
        self.cursor_pos = self.char_len();
    }

    fn move_cursor_word_left(&mut self) {
        let chars: Vec<char> = self.content.chars().collect();
        if self.cursor_pos == 0 {
            return;
        }
        
        // Skip whitespace
        while self.cursor_pos > 0 && chars.get(self.cursor_pos - 1).unwrap_or(&' ').is_whitespace() {
            self.cursor_pos -= 1;
        }
        
        // Skip word characters
        while self.cursor_pos > 0 && !chars.get(self.cursor_pos - 1).unwrap_or(&' ').is_whitespace() {
            self.cursor_pos -= 1;
        }
    }

    fn move_cursor_word_right(&mut self) {
        let chars: Vec<char> = self.content.chars().collect();
        let char_len = chars.len();
        
        if self.cursor_pos >= char_len {
            return;
        }
        
        // Skip word characters
        while self.cursor_pos < char_len && !chars.get(self.cursor_pos).unwrap_or(&' ').is_whitespace() {
            self.cursor_pos += 1;
        }
        
        // Skip whitespace
        while self.cursor_pos < char_len && chars.get(self.cursor_pos).unwrap_or(&' ').is_whitespace() {
            self.cursor_pos += 1;
        }
    }

    fn clear(&mut self) {
        self.content.clear();
        self.cursor_pos = 0;
        self.display_offset = 0;
    }
}

impl TerminalInput {
    pub fn new() -> Self {
        let size = terminal::size().unwrap_or((80, 24));
        Self {
            exit_count: 0,
            esc_count: 0,
            terminal_size: size,
            prompt: "💬 You: ".to_string(),
        }
    }

    pub async fn read_user_input(&mut self) -> Result<InputEvent, Box<dyn std::error::Error>> {
        // Check if we're running interactively
        use std::io::IsTerminal;
        if !std::io::stdin().is_terminal() {
            // Not interactive, return a dummy exit to gracefully handle non-interactive mode
            return Ok(InputEvent::ExitRequest(3));
        }

        let mut buffer = TextBuffer::new();
        let mut stdout = stdout();

        enable_raw_mode()?;
        
        // Update terminal size
        self.terminal_size = terminal::size().unwrap_or(self.terminal_size);

        // Show initial prompt
        self.render_input(&mut stdout, &buffer)?;

        loop {
            if event::poll(std::time::Duration::from_millis(100))? {
                if let Event::Key(key_event) = event::read()? {
                    match key_event {
                        KeyEvent {
                            code: KeyCode::Enter,
                            ..
                        } => {
                            execute!(stdout, Print("\n"))?;
                            disable_raw_mode()?;
                            
                            if buffer.content.trim().is_empty() {
                                enable_raw_mode()?;
                                self.render_input(&mut stdout, &buffer)?;
                                continue;
                            }
                            
                            self.exit_count = 0;
                            self.esc_count = 0;
                            return Ok(InputEvent::UserInput(buffer.content.trim().to_string()));
                        }

                        // Ctrl+C handling
                        KeyEvent {
                            code: KeyCode::Char('c'),
                            modifiers: KeyModifiers::CONTROL,
                            ..
                        } => {
                            self.exit_count += 1;
                            let remaining = 3 - self.exit_count;
                            
                            execute!(
                                stdout,
                                Print("\n"),
                                SetForegroundColor(Color::Yellow),
                                Print(format!("🚪 Exit request {} of 3 ", self.exit_count)),
                            )?;
                            
                            if remaining > 0 {
                                execute!(
                                    stdout,
                                    Print(format!("(Press {} more times to exit)\n", remaining)),
                                    ResetColor
                                )?;
                                self.render_input(&mut stdout, &buffer)?;
                            } else {
                                execute!(stdout, ResetColor, Print("\n"))?;
                                disable_raw_mode()?;
                                return Ok(InputEvent::ExitRequest(self.exit_count));
                            }
                        }

                        // ESC key handling
                        KeyEvent {
                            code: KeyCode::Esc,
                            ..
                        } => {
                            self.esc_count += 1;
                            
                            if self.esc_count >= 3 {
                                buffer.clear();
                                self.esc_count = 0;
                                
                                execute!(
                                    stdout,
                                    Print("\n"),
                                    SetForegroundColor(Color::Cyan),
                                    Print("🧹 Input cleared!\n"),
                                    ResetColor
                                )?;
                                
                                self.render_input(&mut stdout, &buffer)?;
                            } else {
                                execute!(
                                    stdout,
                                    SetForegroundColor(Color::Yellow),
                                    Print(format!(" [ESC: {}/3]", self.esc_count)),
                                    ResetColor
                                )?;
                            }
                        }

                        // Navigation keys
                        KeyEvent {
                            code: KeyCode::Left,
                            modifiers: KeyModifiers::CONTROL,
                            ..
                        } => {
                            buffer.move_cursor_word_left();
                            self.render_input(&mut stdout, &buffer)?;
                        }

                        KeyEvent {
                            code: KeyCode::Right,
                            modifiers: KeyModifiers::CONTROL,
                            ..
                        } => {
                            buffer.move_cursor_word_right();
                            self.render_input(&mut stdout, &buffer)?;
                        }

                        KeyEvent {
                            code: KeyCode::Left,
                            ..
                        } => {
                            buffer.move_cursor_left();
                            self.render_input(&mut stdout, &buffer)?;
                        }

                        KeyEvent {
                            code: KeyCode::Right,
                            ..
                        } => {
                            buffer.move_cursor_right();
                            self.render_input(&mut stdout, &buffer)?;
                        }

                        KeyEvent {
                            code: KeyCode::Home,
                            ..
                        } => {
                            buffer.move_cursor_home();
                            self.render_input(&mut stdout, &buffer)?;
                        }

                        KeyEvent {
                            code: KeyCode::End,
                            ..
                        } => {
                            buffer.move_cursor_end();
                            self.render_input(&mut stdout, &buffer)?;
                        }

                        // Editing keys
                        KeyEvent {
                            code: KeyCode::Backspace,
                            ..
                        } => {
                            if buffer.delete_char_before() {
                                self.esc_count = 0;
                                self.render_input(&mut stdout, &buffer)?;
                            }
                        }

                        KeyEvent {
                            code: KeyCode::Delete,
                            ..
                        } => {
                            if buffer.delete_char_at() {
                                self.esc_count = 0;
                                self.render_input(&mut stdout, &buffer)?;
                            }
                        }

                        // Ctrl+A - Home
                        KeyEvent {
                            code: KeyCode::Char('a'),
                            modifiers: KeyModifiers::CONTROL,
                            ..
                        } => {
                            buffer.move_cursor_home();
                            self.render_input(&mut stdout, &buffer)?;
                        }

                        // Ctrl+E - End
                        KeyEvent {
                            code: KeyCode::Char('e'),
                            modifiers: KeyModifiers::CONTROL,
                            ..
                        } => {
                            buffer.move_cursor_end();
                            self.render_input(&mut stdout, &buffer)?;
                        }

                        // Ctrl+U - Clear line
                        KeyEvent {
                            code: KeyCode::Char('u'),
                            modifiers: KeyModifiers::CONTROL,
                            ..
                        } => {
                            buffer.clear();
                            self.render_input(&mut stdout, &buffer)?;
                        }

                        // Regular character input
                        KeyEvent {
                            code: KeyCode::Char(c),
                            modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
                            ..
                        } => {
                            buffer.insert_char(c);
                            self.esc_count = 0;
                            self.render_input(&mut stdout, &buffer)?;
                        }

                        // Tab - insert 4 spaces
                        KeyEvent {
                            code: KeyCode::Tab,
                            ..
                        } => {
                            for _ in 0..4 {
                                buffer.insert_char(' ');
                            }
                            self.esc_count = 0;
                            self.render_input(&mut stdout, &buffer)?;
                        }

                        _ => {
                            // Reset ESC count on any other key
                            self.esc_count = 0;
                        }
                    }
                }
            }
        }
    }

    fn render_input(&self, stdout: &mut Stdout, buffer: &TextBuffer) -> Result<(), Box<dyn std::error::Error>> {
        let prompt_width = self.prompt.width();
        let available_width = (self.terminal_size.0 as usize).saturating_sub(prompt_width);
        
        let text_chars: Vec<char> = buffer.content.chars().collect();
        
        // Calculate display width up to cursor position
        let cursor_text: String = text_chars[..buffer.cursor_pos].iter().collect();
        let cursor_display_width = cursor_text.width();
        
        // Calculate what part of the text to display and cursor position
        let (display_text, cursor_display_pos) = if buffer.content.width() <= available_width {
            // Text fits entirely on screen
            (buffer.content.clone(), cursor_display_width)
        } else {
            // Text is longer than available width - need horizontal scrolling
            // We need to find a good window that keeps the cursor visible
            
            // Try to keep cursor in the middle of the visible area when possible
            let desired_cursor_pos = available_width / 2;
            
            if cursor_display_width < desired_cursor_pos {
                // Cursor is near the beginning, show from start
                let mut display_width = 0;
                let mut end_char_idx = 0;
                
                for (i, ch) in text_chars.iter().enumerate() {
                    let ch_width = ch.to_string().width();
                    if display_width + ch_width > available_width {
                        break;
                    }
                    display_width += ch_width;
                    end_char_idx = i + 1;
                }
                
                let display_chars: String = text_chars[..end_char_idx].iter().collect();
                (display_chars, cursor_display_width)
            } else {
                // Cursor is further right, need to scroll
                let target_start_width = cursor_display_width.saturating_sub(desired_cursor_pos);
                
                // Find the character index where we should start displaying
                let mut current_width = 0;
                let mut start_char_idx = 0;
                
                for (i, ch) in text_chars.iter().enumerate() {
                    let ch_width = ch.to_string().width();
                    if current_width >= target_start_width {
                        start_char_idx = i;
                        break;
                    }
                    current_width += ch_width;
                }
                
                // Now find how much text we can display from this start position
                let mut display_width = 0;
                let mut end_char_idx = start_char_idx;
                
                for (i, ch) in text_chars[start_char_idx..].iter().enumerate() {
                    let ch_width = ch.to_string().width();
                    if display_width + ch_width > available_width {
                        break;
                    }
                    display_width += ch_width;
                    end_char_idx = start_char_idx + i + 1;
                }
                
                let display_chars: String = text_chars[start_char_idx..end_char_idx].iter().collect();
                
                // Calculate cursor position within the displayed text
                let prefix_text: String = text_chars[start_char_idx..buffer.cursor_pos].iter().collect();
                let cursor_pos_in_display = prefix_text.width();
                
                (display_chars, cursor_pos_in_display)
            }
        };

        // Clear the current line and render new content
        execute!(
            stdout,
            cursor::MoveToColumn(0),
            Clear(ClearType::CurrentLine),
            Print(&self.prompt),
            Print(&display_text),
            cursor::MoveToColumn((prompt_width + cursor_display_pos) as u16)
        )?;

        stdout.flush()?;
        Ok(())
    }

    pub fn reset_counters(&mut self) {
        self.exit_count = 0;
        self.esc_count = 0;
    }
}

impl Drop for TerminalInput {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
    }
}