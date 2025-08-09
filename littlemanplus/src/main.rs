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

use interpreter::interface::TerminalInterface;

fn main() {
    const PROGRAM: &str = r#"
    init   LDA v0
       STA 90
       LDA v1
       STA 91
       LDA v2
       STA 92
       LDA v3
       STA 93
       LDA v4
       STA 94
       LDA v5
       STA 95
       LDA v6
       STA 96
       LDA v7
       STA 97
       LDA v8
       STA 98
       LDA v9
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
v0     DAT 32
v1     DAT 7
v2     DAT 19
v3     DAT 75
v4     DAT 21
v5     DAT 14
v6     DAT 95
v7     DAT 35
v8     DAT 61
v9     DAT 50"#;
    let terminal = ratatui::init();
    let mut tui = TerminalInterface::new();
    tui.set_program(PROGRAM);
    tui.run(terminal);
    ratatui::restore();
}
