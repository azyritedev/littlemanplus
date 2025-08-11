use super::vm::{VirtualMachine, VirtualMachineStep};
use derive_setters::Setters;
use lmp_common::ClonableFn;
use ratatui::crossterm::event;
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::prelude::*;
use ratatui::style::Styled;
use ratatui::widgets::*;
use ratatui::DefaultTerminal;
use std::fmt::Debug;
use std::str::FromStr;
use tui_textarea::{CursorMove, TextArea};

#[derive(Debug)]
pub struct TerminalInterface<'a> {
    vm: VirtualMachine,
    should_exit: bool,

    // State
    program_textarea: TextArea<'a>,
    outputs: Vec<i64>,
    outputs_state: ListState,
    inputs: Vec<i64>,
    inputs_state: ListState,
    memory_state: ListState,
    vm_on: bool,
    // Without WidgetRef, these cannot be Boxed
    current_popup: Option<Popup<'a>>,
    current_modal: Option<Modal<'a>>,
    interface_mode: InterfaceMode,
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
            inputs: Vec::new(),
            inputs_state: ListState::default(),
            memory_state: ListState::default(),
            vm_on: false,
            current_popup: None,
            current_modal: None,
            interface_mode: InterfaceMode::default(),
        }
    }

    pub fn run(mut self, mut terminal: DefaultTerminal) {
        while !self.should_exit {
            // TODO: PERF: optimise rendering using "dirty" system
            terminal.draw(|frame| {
                frame.render_widget(&mut self, frame.area());
                // When WidgetRef is stable, this clone can be removed
                if let Some(modal) = self.current_modal.as_ref().cloned() {
                    let area = frame.area();
                    let modal_area = Rect {
                        x: area.width / 4,
                        y: area.height / 3,
                        width: area.width / 2,
                        height: area.height / 3,
                    };

                    frame.render_widget(modal, modal_area);
                }

                // When WidgetRef is stable, this clone can be removed
                if let Some(popup) = self.current_popup.as_ref().cloned() {
                    let area = frame.area();
                    let popup_area = Rect {
                        x: area.width / 4,
                        y: area.height / 3,
                        width: area.width / 2,
                        height: area.height / 3,
                    };

                    frame.render_widget(popup, popup_area);
                }
            }).unwrap();
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

            // Pause the VM if a modal is up to prevent it from locking up the thread
            // Potentially could be solved by adding "pause" functionality or running it separately
            if self.vm_on && self.current_modal.is_none() {
                match self.vm.step() {
                    VirtualMachineStep::Output(value) => {
                        self.outputs.push(value);
                    }
                    VirtualMachineStep::InputRequired => {
                        // Show input modal
                        let input_modal = Modal::default()
                            .title("Input Required")
                            .description("The virtual machine required input (integer)")
                            .input_title("Input")
                            .validate(Some(
                                Box::new(|inp| {
                                    if let Err(err) = <i64>::from_str(&inp) {
                                        Some(err.to_string())
                                    } else { None }
                                })
                            ));

                        self.current_modal = Some(input_modal);
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
        // Catch all key events if a popup is up
        if self.current_popup.is_some() {
            match key.code {
                KeyCode::Enter => {
                    self.current_popup = None;
                }
                _ => {} // No-op
            }
            return
        }

        let in_modal = self.current_modal.is_some();

        if in_modal {
            // TODO: More general submission logic?
            if key.code == KeyCode::Enter {
                // SAFE UNWRAP: We checked if there was a modal before
                let modal = self.current_modal.take().unwrap();

                let Ok(input) = modal.textarea.lines()[0].parse() else {
                    return
                };
                self.inputs.push(input);
                self.vm.input(input);

                return
            }
        }

        // If in modal, redirect all inputs to the textarea in the modal
        let current_textarea = if let Some(modal) = self.current_modal.as_mut() {
            &mut modal.textarea
        } else {
            &mut self.program_textarea
        };

        if key.code == KeyCode::Esc {
            self.should_exit = true;
        }

        if key.modifiers.contains(event::KeyModifiers::ALT) {
            // Alt + ... keys
            // i.e., selection key combos

            if !current_textarea.is_selecting() {
                current_textarea.start_selection()
            }

            match key.code {
                KeyCode::Left => {
                    current_textarea.move_cursor(CursorMove::Back);
                }
                KeyCode::Right => {
                    current_textarea.move_cursor(CursorMove::Forward);
                }
                _ => {} // No-op
            }
        }

        if key.modifiers.contains(event::KeyModifiers::CONTROL) {
            // Ctrl + ... keys
            match key.code {
                KeyCode::Left => {
                    current_textarea.move_cursor(CursorMove::WordBack);
                }
                KeyCode::Right => {
                    current_textarea.move_cursor(CursorMove::WordForward);
                }

                KeyCode::Char('a') => {
                    current_textarea.select_all();
                }
                KeyCode::Char('c') => {
                    current_textarea.copy();
                }
                KeyCode::Char('x') => {
                    current_textarea.cut();
                }
                KeyCode::Char('p') => {
                    current_textarea.paste();
                }
                KeyCode::Char('r') => {
                    // If VM on, do nothing
                    // If in modal, do nothing
                    if self.vm_on || in_modal {
                        return;
                    }

                    // Run the program
                    if let Err(error) = self.vm.compile(self.program_textarea.lines().join("\n")) {
                        let error_popup = Popup::default().title("Compiler Error")
                            .content(format!("A compiler error occurred: {error}"));
                        self.current_popup = Some(error_popup);
                        return;
                    }

                    self.inputs.clear();
                    self.outputs.clear();

                    self.vm_on = true;
                }
                KeyCode::Char('n') => {
                    if !self.vm_on && !in_modal { // only allow clearing when VM is not on, ignore in modals
                        self.inputs.clear();
                        self.outputs.clear();
                        self.vm.reset();
                    }
                }
                _ => {} // No-op
            }
        } else {
            // Otherwise send input to the textarea
            current_textarea.input(key);
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

        let input_block = Block::bordered().title("Input");

        let input_list_items: Vec<ListItem> = self.inputs.iter().map(|input| {
            ListItem::new(vec![
                format!("{}", input).into()
            ])
        }).collect();

        let input_list = List::new(input_list_items).block(input_block);
        StatefulWidget::render(input_list, input_area, buf, &mut self.inputs_state);

        let output_block = Block::bordered().title("Output");

        let output_list_items: Vec<ListItem> = self.outputs.iter().map(|output| {
            ListItem::new(vec![
                format!("{}", output).into()
            ])
        }).collect();

        let output_list = List::new(output_list_items).block(output_block);
        StatefulWidget::render(output_list, output_area, buf, &mut self.outputs_state);

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
                " | ".fg(Color::DarkGray),
            ])
        ]).block(block).render(area, buf);
    }
}

#[derive(Debug, Default, Setters, Clone)]
struct Popup<'a> {
    #[setters(into)]
    title: Line<'a>,
    #[setters(into)]
    content: Text<'a>,
    border_style: Style,
    title_style: Style,
    style: Style,
}

impl Widget for Popup<'_> {
    fn render(mut self, area: Rect, buf: &mut Buffer) {
        // Add line break + exit instructions
        self.content.push_line(Line::default());
        self.content.push_line(Line::from(vec![
            "Press ".into(),
            "Enter".fg(Color::Black).bg(Color::White).into(),
            " to dismiss".into(),
        ]));

        Clear.render(area, buf);
        let block = Block::bordered()
            .title(self.title)
            .title_style(self.title_style)
            .border_style(self.border_style)
            .padding(Padding::new(1, 1, 0, 0));
        Paragraph::new(self.content)
            .wrap(Wrap { trim: true })
            .style(self.style)
            .block(block)
            .render(area, buf);
    }
}

#[derive(Debug, Clone, Default, Setters)]
struct Modal<'a> {
    #[setters(into)]
    title: Line<'a>,
    #[setters(into)]
    description: Line<'a>,
    #[setters(into)]
    input_title: Line<'a>,

    border_style: Style,
    title_style: Style,

    pub textarea: TextArea<'a>,

    /// Optional validator function
    ///
    /// Should return an error message or None for success
    #[setters]
    validate: Option<Box<dyn ClonableFn<String, Option<String>>>>,
}

impl Widget for Modal<'_> {
    fn render(mut self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);

        // Run validation function if provided, default to always valid (None)
        let is_invalid = self.validate
            .as_ref()
            .map(|validate| validate(self.textarea.lines().join("\n")))
            .unwrap_or(None);

        let outer_block = Block::bordered()
            .title(self.title)
            .title_style(self.title_style)
            .border_style(self.border_style)
            .padding(Padding::new(1, 1, 0, 0));

        let [description_area, text_area, status_area] = Layout::vertical([
            Constraint::Length(2),
            Constraint::Length(3),
            Constraint::Fill(1)
        ]).areas(outer_block.inner(area));

        // Render description
        Paragraph::new(self.description).render(description_area, buf);

        let text_block_style = if is_invalid.is_some() { Style::default().fg(Color::Red) } else { Style::default() };
        let text_block = Block::bordered().title(self.input_title).set_style(text_block_style);

        // Alter styling if invalid
        if let Some(reason) = is_invalid {
            self.textarea.set_style(Style::default().fg(Color::Red));
            Paragraph::new(vec![
                Line::from(vec![
                    "Invalid input: ".bold(),
                    reason.into()
                ]).fg(Color::Red)
            ]).render(status_area, buf);
        }

        self.textarea.set_block(text_block);
        self.textarea.set_cursor_line_style(Style::default());

        self.textarea.render(text_area, buf);

        outer_block.render(area, buf);
    }
}

#[derive(Debug, Default)]
enum InterfaceMode {
    #[default]
    Program,
    Configuration,
}
