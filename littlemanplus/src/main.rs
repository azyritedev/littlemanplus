mod interpreter;

use interpreter::interface::TerminalInterface;

fn main() {
    let terminal = ratatui::init();
    TerminalInterface::new().run(terminal);
    ratatui::restore();
}
