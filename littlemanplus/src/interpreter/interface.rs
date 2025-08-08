use super::vm::{VirtualMachine, VirtualMachineStep};
use ratatui::crossterm::event;
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::prelude::*;
use ratatui::widgets::*;
use ratatui::DefaultTerminal;
use tui_textarea::{CursorMove, TextArea};

#[derive(Debug)]
pub struct TerminalInterface<'a> {
    vm: VirtualMachine,
    should_exit: bool,

    // State
    program_textarea: TextArea<'a>,
    outputs: Vec<i64>,
    outputs_state: ListState,
    memory_state: ListState,
    vm_on: bool,
}

// See other impl for rendering logic
impl TerminalInterface<'_> {
    pub fn new() -> Self {
        // Textarea styling
        let mut program_textarea = TextArea::default();
        let textarea_block = Block::bordered().title("Program");
        program_textarea.set_block(textarea_block);
        program_textarea.set_line_number_style(Style::default().fg(Color::DarkGray));
        program_textarea.set_cursor_line_style(Style::default());

        Self {
            vm: VirtualMachine::new(),
            should_exit: false,
            program_textarea,
            outputs: Vec::new(),
            outputs_state: ListState::default(),
            memory_state: ListState::default(),
            vm_on: false,
        }
    }

    pub fn run(mut self, mut terminal: DefaultTerminal) {
        while !self.should_exit {
            terminal.draw(|frame| frame.render_widget(&mut self, frame.area())).unwrap();
            // Do not block when there aren't any events to read
            if let Ok(true) = event::poll(core::time::Duration::from_secs(0)) {
                if let Event::Key(event) = event::read().unwrap() {
                    self.handle_key(event);
                }
            }

            // Step the vm, if on
            if self.vm_on && self.vm.halted() {
                self.vm_on = false;
            }

            if self.vm_on {
                match self.vm.step() {
                    VirtualMachineStep::Output(value) => {
                        self.outputs.push(value);
                    }
                    VirtualMachineStep::InputRequired => {
                        self.vm.input(10);
                    }
                    _ => {}
                }
            }
        }
    }

    /// Set the currently loaded program in the interface. Does not run it!
    pub fn set_program<S: AsRef<str>>(&mut self, program: S) {
        // Bit hacky, but avoids having us split the text by newlines
        self.program_textarea.set_yank_text(program.as_ref().trim());
        self.program_textarea.paste();
        self.program_textarea.set_yank_text("");
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if key.code == KeyCode::Esc {
            self.should_exit = true;
        }

        if key.modifiers.contains(event::KeyModifiers::ALT) {
            // Alt + ... keys
            // i.e., selection key combos

            if !self.program_textarea.is_selecting() {
                self.program_textarea.start_selection()
            }

            match key.code {
                KeyCode::Left => {
                    self.program_textarea.move_cursor(CursorMove::Back);
                }
                KeyCode::Right => {
                    self.program_textarea.move_cursor(CursorMove::Forward);
                }
                _ => {} // No-op
            }
        }

        if key.modifiers.contains(event::KeyModifiers::CONTROL) {
            // Ctrl + ... keys
            match key.code {
                KeyCode::Left => {
                    self.program_textarea.move_cursor(CursorMove::WordBack);
                }
                KeyCode::Right => {
                    self.program_textarea.move_cursor(CursorMove::WordForward);
                }

                KeyCode::Char('a') => {
                    self.program_textarea.select_all();
                }
                KeyCode::Char('c') => {
                    self.program_textarea.copy();
                }
                KeyCode::Char('x') => {
                    self.program_textarea.cut();
                }
                KeyCode::Char('p') => {
                    self.program_textarea.paste();
                }
                KeyCode::Char('r') => {
                    // Run the program
                    if let Err(error) = self.vm.compile(self.program_textarea.lines().join("\n")) {
                        // TODO: handle error
                        self.outputs.push(999);
                        return;
                    }

                    // Reset outputs
                    self.outputs.clear();

                    self.vm_on = true;
                }
                KeyCode::Char('n') => {
                    if !self.vm_on { // only allow clearing when VM is not on
                        self.outputs.clear();
                        self.vm.reset();
                    }
                }
                _ => {} // No-op
            }
        } else {
            // Otherwise send input to the textarea
            self.program_textarea.input(key);
        }
    }
}

// Allow TerminalInterface to be rendered
impl Widget for &mut TerminalInterface<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let [header_area, main_area, footer_area] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Fill(1),
            Constraint::Length(3),
        ]).areas(area);

        let [program_area, cpu_io_area, ram_area, config_area] = Layout::horizontal([
            Constraint::Ratio(2, 6),
            Constraint::Ratio(2, 6),
            Constraint::Ratio(1, 6),
            Constraint::Ratio(1, 6),
        ]).areas(main_area);

        let [cpu_area, io_area] = Layout::vertical([
            Constraint::Ratio(1, 2),
            Constraint::Ratio(1, 2),
        ]).areas(cpu_io_area);

        self.render_program(program_area, buf);
        self.render_header(header_area, buf);
        self.render_cpu(cpu_area, buf);
        self.render_io(io_area, buf);
        self.render_ram(ram_area, buf);
        self.render_footer(footer_area, buf);
        self.render_config(config_area, buf);
    }
}

// Rendering methods
impl TerminalInterface<'_> {
    fn render_program(&self, area: Rect, buf: &mut Buffer) {
        self.program_textarea.render(area, buf);
    }

    fn render_header(&self, area: Rect, buf: &mut Buffer) {
        Paragraph::new(vec![
            "Little Man Plus".bold().into_centered_line()
        ]).block(Block::bordered().border_type(BorderType::Double).cyan()).render(area, buf);
    }

    fn render_cpu(&self, area: Rect, buf: &mut Buffer) {
        let outer_block = Block::bordered().title("Central Processing Unit");

        let [status_area, stats_area] = Layout::vertical([Constraint::Length(2), Constraint::Length(3)]).areas(outer_block.inner(area));
        Paragraph::new(vec![
            if self.vm_on { "VM Running".bold().fg(Color::Green).into() } else { "VM Halted".fg(Color::Red).bold().into() }
        ]).render(status_area, buf);
        let [program_counter_area, accumulator_area, cycles_area] = Layout::horizontal([
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
        ]).areas(stats_area);

        let program_counter_block = Block::bordered().title("Program Counter");
        Paragraph::new(vec![
            format!("{}", self.vm.program_counter()).into()
        ]).block(program_counter_block).render(program_counter_area, buf);

        let accumulator_block = Block::bordered().title("Accumulator");
        Paragraph::new(vec![
            format!("{}", self.vm.accumulator()).into()
        ]).block(accumulator_block).render(accumulator_area, buf);

        let cycles_block = Block::bordered().title("Cycles");
        Paragraph::new(vec![
            format!("{}", self.vm.cycles()).into()
        ]).block(cycles_block).render(cycles_area, buf);

        outer_block.render(area, buf);
    }

    fn render_io(&mut self, area: Rect, buf: &mut Buffer) {
        let outer_block = Block::default().title("I/O").title_alignment(Alignment::Center);

        let [input_area, output_area] = Layout::horizontal([
            Constraint::Ratio(1, 2),
            Constraint::Ratio(1, 2),
        ]).areas(outer_block.inner(area));

        let input_block = Block::bordered().title("Input").render(input_area, buf);

        let output_block = Block::bordered().title("Output");

        let list_items: Vec<ListItem> = self.outputs.iter().map(|output| {
            ListItem::new(vec![
                format!("{}", output).into()
            ])
        }).collect();

        let list = List::new(list_items).block(output_block);
        StatefulWidget::render(list, output_area, buf, &mut self.outputs_state);

        outer_block.render(area, buf);
    }

    fn render_ram(&mut self, area: Rect, buf: &mut Buffer) {
        let outer_block = Block::bordered().title("Memory");

        let list_items: Vec<ListItem> = self.vm.memory().iter().enumerate().map(|(addr, cell)| {
            ListItem::new(vec![
                format!("{:<3}{addr:0>3}: {}", if self.vm.program_counter() == addr { ">>" } else { "" }, cell.data).into(),
            ])
        }).collect();

        let list = List::new(list_items).block(outer_block).highlight_style(Style::default().fg(Color::Black).bg(Color::White));
        // Select the last accessed address
        let accessing = self.vm.accessing();
        self.memory_state.select(Some(accessing));

        StatefulWidget::render(list, area, buf, &mut self.memory_state);
    }

    fn render_config(&mut self, area: Rect, buf: &mut Buffer) {
        let outer_block = Block::bordered().title("Configuration");

        outer_block.render(area, buf);
    }

    fn render_footer(&mut self, area: Rect, buf: &mut Buffer) {
        let block = Block::bordered();

        Paragraph::new(vec![
            Line::from(vec![
                "Ctrl+R".fg(Color::Black).bg(Color::White),
                " Run Program ".into(),
                "Ctrl+N".fg(Color::Black).bg(Color::White),
                " Reset VM ".into(),
            ])
        ]).block(block).render(area, buf);
    }
}

struct TerminalGrid {
    cols: usize,
    rows: usize,
}

impl TerminalGrid {
    fn new(cols: usize, rows: usize) -> Self {
        Self { cols, rows }
    }
}

impl Widget for TerminalGrid {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let col_constraints = (0..self.cols).map(|_| Constraint::Length(9));
        let row_constraints = (0..self.rows).map(|_| Constraint::Length(2));
        let horizontal = Layout::horizontal(col_constraints).spacing(0);
        let vertical = Layout::vertical(row_constraints).spacing(0);

        let rows = vertical.split(area);
        let cells = rows.iter().flat_map(|&row| horizontal.split(row).to_vec());

        for (i, cell) in cells.enumerate() {
            Paragraph::new(vec![
                format!("{:02}", i + 1).into(),
                "012345".into()
            ])
                .block(Block::default())
                .render(cell, buf);
        }
    }
}