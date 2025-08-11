/*
    littlemanplus — a Rust-based Little Man Computer simulator
    Copyright (C) 2025 azyrite

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */
mod interpreter;
mod config;

use interpreter::interface::TerminalInterface;

fn main() {
    const PROGRAM: &str = r#"
       INP
       STA 90
       INP
       STA 91
       INP
       STA 92
       INP
       STA 93
       INP
       STA 94
       INP
       STA 95
       INP
       STA 96
       INP
       STA 97
       INP
       STA 98
       INP
       STA 99
loop   LDA true
       STA sorted
       LDA first
       STA pos
       ADD one
       STA next
step   LDA @pos
       SUB @next
       BRZ pass
       BRP swap
pass   LDA pos
       ADD one
       STA pos
       LDA next
       ADD one
       STA next
       LDA pos
       SUB last
       BRZ repeat
       BRA step
swap   LDA @next
       STA temp
       LDA @pos
       STA @next
       LDA temp
       STA @pos
       LDA false
       STA sorted
       BRA pass
repeat LDA sorted
       SUB true
       BRZ exit
       BRA loop
exit   LDA first
       STA pos
outs   LDA @pos
       OUT
       LDA pos
       ADD one
       STA pos
       LDA last
       SUB pos
       BRP outs
       HLT
pos    DAT
next   DAT
temp   DAT
sorted DAT 0
true   DAT 1
false  DAT 0
one    DAT 1
first  DAT 90
last   DAT 99
"#;
    let terminal = ratatui::init();
    let mut tui = TerminalInterface::new();
    tui.set_program(PROGRAM);
    tui.run(terminal);
    ratatui::restore();
}
