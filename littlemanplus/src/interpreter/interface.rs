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
        let [header_area, main_area] = Layout::vertical([
            Constraint::Ratio(1, 5),
            Constraint::Fill(1)
        ]).areas(area);

        let [program_area, cpu_io_area, ram_area] = Layout::horizontal([
            Constraint::Ratio(1, 6),
            Constraint::Ratio(2, 6),
            Constraint::Ratio(3, 6),
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
        ]).block(Block::bordered()).render(area, buf);
    }

    fn render_cpu(&self, area: Rect, buf: &mut Buffer) {
        let outer_block = Block::bordered().title("Central Processing Unit");

        let [status_area, stats_area] = Layout::vertical([Constraint::Ratio(1, 6),Constraint::Ratio(1, 3)]).areas(outer_block.inner(area));
        Paragraph::new(vec![
            if self.vm_on { "VM ON".bold().fg(Color::Green).into() } else { "VM OFF".fg(Color::Red).bold().into() }
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

        // TODO: Render RAM

        outer_block.render(area, buf);
    }
}