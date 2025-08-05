mod interpreter;

use interpreter::vm::VirtualMachine;

fn main() {
    let program = r#"
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

exit   OUT
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
v9     DAT 50
"#;
    let mut vm = VirtualMachine::new();

    let outcome = vm.compile_run(program);
    println!("{:?}", outcome);
}
