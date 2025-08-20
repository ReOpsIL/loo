use crate::autocomplete::{AutocompleteEngine, FileEntry};
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
    autocomplete_engine: AutocompleteEngine,
}

#[derive(Debug)]
enum AutocompleteState {
    None,
    FileSystem {
        suggestions: Vec<FileEntry>,
        selected_index: usize,
        prefix: String,
        start_pos: usize, // Position in text where @ starts
    },
    Command {
        suggestions: Vec<String>,
        selected_index: usize,
        prefix: String,
        start_pos: usize, // Position in text where / starts
    },
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
    autocomplete_state: AutocompleteState,
}

impl TextBuffer {
    fn new() -> Self {
        Self {
            content: String::new(),
            cursor_pos: 0,
            display_offset: 0,
            autocomplete_state: AutocompleteState::None,
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
        self.autocomplete_state = AutocompleteState::None;
    }

    // Check if current position triggers autocomplete
    fn should_show_autocomplete(&self) -> bool {
        if self.cursor_pos == 0 {
            return false;
        }

        let chars: Vec<char> = self.content.chars().collect();
        let mut pos = self.cursor_pos;

        // Look backwards for @ or / at word boundary
        while pos > 0 {
            pos -= 1;
            let ch = chars[pos];
            
            if ch == '@' || ch == '/' {
                // Check if it's at start or preceded by whitespace
                if pos == 0 || chars[pos - 1].is_whitespace() {
                    return true;
                }
            }
            
            if ch.is_whitespace() {
                break;
            }
        }
        
        false
    }

    // Extract the autocomplete prefix (text after @ or /)
    fn get_autocomplete_prefix(&self) -> Option<(char, usize, String)> {
        if self.cursor_pos == 0 {
            return None;
        }

        let chars: Vec<char> = self.content.chars().collect();
        let mut start_pos = self.cursor_pos;

        // Look backwards for @ or /
        while start_pos > 0 {
            start_pos -= 1;
            let ch = chars[start_pos];
            
            if ch == '@' || ch == '/' {
                // Check if it's at start or preceded by whitespace
                if start_pos == 0 || chars[start_pos - 1].is_whitespace() {
                    let prefix: String = chars[start_pos + 1..self.cursor_pos].iter().collect();
                    return Some((ch, start_pos, prefix));
                }
            }
            
            if ch.is_whitespace() {
                break;
            }
        }
        
        None
    }

    // Complete autocomplete selection - replace @ or / prefix with selected item
    fn complete_autocomplete(&mut self, completion_text: &str, is_directory: bool) -> bool {
        if let Some((_trigger_char, start_pos, _prefix)) = self.get_autocomplete_prefix() {
            // Remove the text from trigger character to cursor
            let start_byte = self.char_to_byte_pos(start_pos + 1); // Skip the @ or /
            let cursor_byte = self.char_to_byte_pos(self.cursor_pos);
            
            // Replace the text between trigger and cursor with completion
            self.content.replace_range(start_byte..cursor_byte, completion_text);
            
            // Update cursor position to be at the end of the completion
            self.cursor_pos = start_pos + 1 + completion_text.chars().count();
            
            // If it's not a directory, clear autocomplete state
            if !is_directory {
                self.autocomplete_state = AutocompleteState::None;
            }
            
            return is_directory;
        }
        false
    }
}

impl TerminalInput {
    pub fn new(working_dir: String) -> Self {
        let size = terminal::size().unwrap_or((80, 24));
        Self {
            exit_count: 0,
            esc_count: 0,
            terminal_size: size,
            prompt: "💬 You: ".to_string(),
            autocomplete_engine: AutocompleteEngine::new(working_dir),
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
                            // Check if autocomplete is active and complete the selection
                            let completion_info = match &buffer.autocomplete_state {
                                AutocompleteState::FileSystem { suggestions, selected_index, .. } => {
                                    if !suggestions.is_empty() && *selected_index < suggestions.len() {
                                        let selected_file = &suggestions[*selected_index];
                                        // Fix double slash issue by not adding / if it already ends with /
                                        let completion_text = if selected_file.is_directory {
                                            if selected_file.full_path.ends_with('/') {
                                                selected_file.full_path.clone()
                                            } else {
                                                format!("{}/", selected_file.full_path)
                                            }
                                        } else {
                                            selected_file.full_path.clone()
                                        };
                                        Some((completion_text, selected_file.is_directory))
                                    } else {
                                        None
                                    }
                                }
                                AutocompleteState::Command { suggestions, selected_index, .. } => {
                                    if !suggestions.is_empty() && *selected_index < suggestions.len() {
                                        Some((suggestions[*selected_index].clone(), false))
                                    } else {
                                        None
                                    }
                                }
                                AutocompleteState::None => None,
                            };

                            if let Some((text, is_directory)) = completion_info {
                                let continue_browsing = buffer.complete_autocomplete(&text, is_directory);
                                
                                if continue_browsing {
                                    // Update autocomplete to show folder contents
                                    self.update_autocomplete(&mut buffer)?;
                                    self.render_with_autocomplete(&mut stdout, &buffer)?;
                                } else {
                                    self.render_input(&mut stdout, &buffer)?;
                                }
                                continue;
                            }

                            // Normal Enter handling - submit the input
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

                        // Up arrow - navigate autocomplete
                        KeyEvent {
                            code: KeyCode::Up,
                            ..
                        } => {
                            match &mut buffer.autocomplete_state {
                                AutocompleteState::FileSystem { selected_index, suggestions, .. } => {
                                    if !suggestions.is_empty() && *selected_index > 0 {
                                        *selected_index -= 1;
                                        self.render_with_autocomplete(&mut stdout, &buffer)?;
                                    }
                                }
                                AutocompleteState::Command { selected_index, suggestions, .. } => {
                                    if !suggestions.is_empty() && *selected_index > 0 {
                                        *selected_index -= 1;
                                        self.render_with_autocomplete(&mut stdout, &buffer)?;
                                    }
                                }
                                AutocompleteState::None => {
                                    // No autocomplete active, ignore
                                }
                            }
                        }

                        // Down arrow - navigate autocomplete
                        KeyEvent {
                            code: KeyCode::Down,
                            ..
                        } => {
                            match &mut buffer.autocomplete_state {
                                AutocompleteState::FileSystem { selected_index, suggestions, .. } => {
                                    if !suggestions.is_empty() && *selected_index < suggestions.len() - 1 {
                                        *selected_index += 1;
                                        self.render_with_autocomplete(&mut stdout, &buffer)?;
                                    }
                                }
                                AutocompleteState::Command { selected_index, suggestions, .. } => {
                                    if !suggestions.is_empty() && *selected_index < suggestions.len() - 1 {
                                        *selected_index += 1;
                                        self.render_with_autocomplete(&mut stdout, &buffer)?;
                                    }
                                }
                                AutocompleteState::None => {
                                    // No autocomplete active, ignore
                                }
                            }
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
                                
                                // Store previous autocomplete state to detect changes
                                let had_autocomplete = !matches!(buffer.autocomplete_state, AutocompleteState::None);
                                
                                // Update autocomplete after deletion
                                self.update_autocomplete(&mut buffer)?;
                                
                                let has_autocomplete = !matches!(buffer.autocomplete_state, AutocompleteState::None);
                                
                                // If autocomplete disappeared, clear the screen first
                                if had_autocomplete && !has_autocomplete {
                                    execute!(stdout, Clear(ClearType::FromCursorDown))?;
                                }
                                
                                if has_autocomplete {
                                    self.render_with_autocomplete(&mut stdout, &buffer)?;
                                } else {
                                    self.render_input(&mut stdout, &buffer)?;
                                }
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
                            // Special handling for space - always hides autocomplete
                            if c == ' ' && matches!(buffer.autocomplete_state, AutocompleteState::FileSystem { .. } | AutocompleteState::Command { .. }) {
                                // Space - cancel autocomplete and insert space
                                buffer.autocomplete_state = AutocompleteState::None;
                                buffer.insert_char(c);
                                // Clear the screen from cursor down to remove autocomplete display
                                execute!(stdout, Clear(ClearType::FromCursorDown))?;
                                self.render_input(&mut stdout, &buffer)?;
                            } 
                            // Ignore @ when already in file system autocomplete mode
                            else if c == '@' && matches!(buffer.autocomplete_state, AutocompleteState::FileSystem { .. }) {
                                // Ignore the @ key when already browsing files
                                continue;
                            }
                            else {
                                buffer.insert_char(c);
                                self.esc_count = 0;
                                
                                // Check for autocomplete triggers
                                self.update_autocomplete(&mut buffer)?;
                                
                                if matches!(buffer.autocomplete_state, AutocompleteState::None) {
                                    self.render_input(&mut stdout, &buffer)?;
                                } else {
                                    self.render_with_autocomplete(&mut stdout, &buffer)?;
                                }
                            }
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

    fn update_autocomplete(&mut self, buffer: &mut TextBuffer) -> Result<(), Box<dyn std::error::Error>> {
        if buffer.should_show_autocomplete() {
            if let Some((trigger_char, start_pos, prefix)) = buffer.get_autocomplete_prefix() {
                match trigger_char {
                    '@' => {
                        // File system autocomplete
                        let suggestions = self.autocomplete_engine.get_file_suggestions(&prefix);
                        buffer.autocomplete_state = AutocompleteState::FileSystem {
                            suggestions,
                            selected_index: 0,
                            prefix: prefix.clone(),
                            start_pos,
                        };
                    }
                    '/' => {
                        // Command autocomplete
                        let mut commands = vec![
                            "model".to_string(),
                            "list-models".to_string(),
                        ];
                        
                        // Filter commands based on prefix
                        commands.retain(|cmd| cmd.starts_with(&prefix));
                        
                        buffer.autocomplete_state = AutocompleteState::Command {
                            suggestions: commands,
                            selected_index: 0,
                            prefix: prefix.clone(),
                            start_pos,
                        };
                    }
                    _ => {
                        buffer.autocomplete_state = AutocompleteState::None;
                    }
                }
            } else {
                buffer.autocomplete_state = AutocompleteState::None;
            }
        } else {
            buffer.autocomplete_state = AutocompleteState::None;
        }
        Ok(())
    }

    fn render_with_autocomplete(&self, stdout: &mut Stdout, buffer: &TextBuffer) -> Result<(), Box<dyn std::error::Error>> {
        // Clear screen from current line down and render the input
        execute!(
            stdout,
            Clear(ClearType::FromCursorDown),
        )?;
        self.render_input(stdout, buffer)?;

        // Then render autocomplete suggestions below
        match &buffer.autocomplete_state {
            AutocompleteState::FileSystem { suggestions, selected_index, .. } => {
                if !suggestions.is_empty() {
                    let max_items = std::cmp::min(suggestions.len(), 10); // Show max 10 items
                    for (i, file_entry) in suggestions.iter().take(max_items).enumerate() {
                        let marker = if i == *selected_index { "> " } else { "  " };
                        let suffix = if file_entry.is_directory { "/" } else { "" };
                        
                        execute!(
                            stdout,
                            Print("\n"),
                            cursor::MoveToColumn(0),
                            SetForegroundColor(if i == *selected_index { Color::White } else { Color::DarkGrey }),
                            Print(format!("{}{}{}", marker, file_entry.name, suffix)),
                            ResetColor
                        )?;
                    }
                    
                    if suggestions.len() > max_items {
                        execute!(
                            stdout,
                            Print("\n"),
                            cursor::MoveToColumn(0),
                            SetForegroundColor(Color::DarkGrey),
                            Print(format!("  ... and {} more", suggestions.len() - max_items)),
                            ResetColor
                        )?;
                    }
                }
            }
            
            AutocompleteState::Command { suggestions, selected_index, .. } => {
                if !suggestions.is_empty() {
                    let command_descriptions = std::collections::HashMap::from([
                        ("model".to_string(), "Change the current LLM model".to_string()),
                        ("list-models".to_string(), "List all available LLM models".to_string()),
                    ]);
                    
                    let max_items = std::cmp::min(suggestions.len(), 8); // Show max 8 commands
                    for (i, command) in suggestions.iter().take(max_items).enumerate() {
                        let marker = if i == *selected_index { "> " } else { "  " };
                        let empty_desc = String::new();
                        let description = command_descriptions.get(command).unwrap_or(&empty_desc);
                        
                        execute!(
                            stdout,
                            Print("\n"),
                            cursor::MoveToColumn(0),
                            SetForegroundColor(if i == *selected_index { Color::White } else { Color::DarkGrey }),
                            Print(format!("{}/{}", marker, command)),
                            ResetColor
                        )?;
                        
                        if !description.is_empty() {
                            // Calculate padding to align descriptions
                            let command_width = command.len() + 3; // "/" + command + some padding
                            let padding = if command_width < 20 { 20 - command_width } else { 4 };
                            let spaces = " ".repeat(padding);
                            
                            execute!(
                                stdout,
                                SetForegroundColor(Color::DarkGrey),
                                Print(format!("{}{}", spaces, description)),
                                ResetColor
                            )?;
                        }
                    }
                }
            }
            
            AutocompleteState::None => {
                // No autocomplete, just render normal input - already done above
            }
        }

        // Move cursor back to the correct position in the input line
        let prompt_width = self.prompt.width();
        let text_chars: Vec<char> = buffer.content.chars().collect();
        let cursor_text: String = text_chars[..buffer.cursor_pos].iter().collect();
        let cursor_display_width = cursor_text.width();
        
        execute!(
            stdout,
            cursor::MoveUp(match &buffer.autocomplete_state {
                AutocompleteState::FileSystem { suggestions, .. } => {
                    let lines = if suggestions.is_empty() { 0 } else {
                        std::cmp::min(suggestions.len(), 10) + if suggestions.len() > 10 { 1 } else { 0 }
                    };
                    lines as u16
                }
                AutocompleteState::Command { suggestions, .. } => {
                    let lines = if suggestions.is_empty() { 0 } else {
                        std::cmp::min(suggestions.len(), 8)
                    };
                    lines as u16
                }
                AutocompleteState::None => 0,
            }),
            cursor::MoveToColumn((prompt_width + cursor_display_width) as u16)
        )?;

        stdout.flush()?;
        Ok(())
    }
}

impl Drop for TerminalInput {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
    }
}