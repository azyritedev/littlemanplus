//! Assembly compiler

use chumsky::prelude::*;
use lmp_common::assembly::Instruction;
use lmp_common::MEMORY_SIZE;
use std::collections::HashMap;

/// Data attached to each instruction in the **unprocessed** AST
#[derive(Debug, Clone)]
enum NodeInstructionData<'a> {
    Pointer(&'a str),
    Label(&'a str),
    Num(i64),
}

impl<'a> From<&'a str> for NodeInstructionData<'a> {
    fn from(s: &'a str) -> Self {
        if s.starts_with('@') {
            Self::Pointer(&s[1..])
        } else {
            Self::Label(s)
        }
    }
}

impl From<i64> for NodeInstructionData<'_> {
    fn from(n: i64) -> Self {
        Self::Num(n)
    }
}

type NodeInstruction<'a> = Instruction<NodeInstructionData<'a>>;

/// Node in the AST
#[derive(Debug)]
struct Node<'a> {
    pub label: Option<&'a str>,
    pub instruction: NodeInstruction<'a>
}

fn num<'a>() -> impl Parser<'a, &'a str, i64> {
    text::int(10).map(|s: &str| s.parse().unwrap())
}

fn label<'a>() -> impl Parser<'a, &'a str, &'a str> {
    // HACK: Use rewind?
    // If all uppercase then reject as it is probably an opcode (e.g., ADD),
    // this prevents label from "eating" the opcode when no label is provided
    opt_whitespace().ignore_then(text::ascii::ident().filter(|s: &&str| {
        !s.is_empty() && !s.chars().all(|c| c.is_ascii_uppercase())
    }))
}

/// at least one whitespace excl. newlines
fn whitespace<'a>() -> impl Parser<'a, &'a str, ()> {
    text::inline_whitespace().at_least(1)
}

/// whitespace (optional) excl. newlines
fn opt_whitespace<'a>() -> impl Parser<'a, &'a str, ()> {
    text::inline_whitespace().at_least(0)
}

/// Input that goes after an instruction
fn instruction_input<'a>() -> impl Parser<'a, &'a str, NodeInstructionData<'a>> {
    // Based off text::ascii::ident but with @ symbols permitted as the first char
    // for pointers
    let text_input = regex("[a-zA-Z_@][a-zA-Z0-9_]*");
    num().map(|n| n.into()).or(text_input.map(|i: &str| i.into()))
}

fn instruction<'a>() -> impl Parser<'a, &'a str, NodeInstruction<'a>> {
    // Allow any indent
    text::inline_whitespace().ignore_then(choice((
        just("INP").to(Instruction::INP),
        just("OUT").to(Instruction::OUT),
        just("HLT").to(Instruction::HLT),
        just("BWN").to(Instruction::BWN),
        just("LDR").to(Instruction::LDR),
        just("DAT").ignore_then(
            whitespace()
                .ignore_then(num())
                .or_not()
                .map(|data| Instruction::DAT(data.unwrap_or_default().into())),
        ),
        just("ADD")
            .ignore_then(whitespace().ignore_then(instruction_input()))
            .map(|data| Instruction::ADD(data)),
        just("SUB")
            .ignore_then(whitespace().ignore_then(instruction_input()))
            .map(|data| Instruction::SUB(data)),
        just("STA")
            .ignore_then(whitespace().ignore_then(instruction_input()))
            .map(|data| Instruction::STA(data)),
        just("LDA")
            .ignore_then(whitespace().ignore_then(instruction_input()))
            .map(|data| Instruction::LDA(data)),
        just("BRA")
            .ignore_then(whitespace().ignore_then(instruction_input()))
            .map(|data| Instruction::BRA(data)),
        just("BRZ")
            .ignore_then(whitespace().ignore_then(instruction_input()))
            .map(|data| Instruction::BRZ(data)),
        just("BRP")
            .ignore_then(whitespace().ignore_then(instruction_input()))
            .map(|data| Instruction::BRP(data)),
        just("BWA")
            .ignore_then(whitespace().ignore_then(instruction_input()))
            .map(|data| Instruction::BWA(data)),
        just("BWO")
            .ignore_then(whitespace().ignore_then(instruction_input()))
            .map(|data| Instruction::BWO(data)),
        just("BWX")
            .ignore_then(whitespace().ignore_then(instruction_input()))
            .map(|data| Instruction::BWX(data)),
    )))
}

fn labeled_line<'a>() -> impl Parser<'a, &'a str, Node<'a>> {
    // FIXME: allow empty lines with trailing whitespace
    let maybe_label = label()
        .then_ignore(whitespace())
        .map(Some)
        .or_not()
        .map(|opt| opt.flatten());

    maybe_label
        .then(instruction())
        .map(|(label, instruction)| Node {
            label,
            instruction
        })
        .then_ignore(text::inline_whitespace())
}

fn parse<'a>() -> impl Parser<'a, &'a str, Vec<Node<'a>>> {
    labeled_line()
        .separated_by(text::newline().repeated().at_least(1))
        .allow_trailing()
        .allow_leading()
        .collect::<Vec<_>>()
        .then_ignore(end())
}

fn resolve_labels(ast: Vec<Node>) -> Result<Vec<Instruction<i64>>, ()> {
    let mut labels = HashMap::new();

    // FIRST PASS: grab labels
    for (addr, expr) in ast.iter().enumerate() {
        if let Some(label) = expr.label {
            labels.insert(label, addr);
        }
    }

    // SECOND PASS: validate and insert mem address
    Ok(ast.into_iter().map(|node| {
        use Instruction::*;
        macro_rules! label_to_addr {
            ($($member:ident),*) => {
                match node.instruction {
                $(
                    $member(NodeInstructionData::Num(n)) => {
                        $member(n)
                    },
                    $member(NodeInstructionData::Label(l)) => {
                        let addr = *labels.get(l).expect(&format!("invalid label {l}"));
                        $member(addr as i64)
                    },
                    $member(NodeInstructionData::Pointer(p)) => {
                        let addr = *labels.get(p).expect(&format!("invalid pointer label {p}")) + MEMORY_SIZE;
                        $member(addr as i64)
                    }
                )*
                    DAT(NodeInstructionData::Num(n)) => DAT(n),
                    BWN => BWN,
                    INP => INP,
                    OUT => OUT,
                    HLT => HLT,
                    _ => panic!("invalid instruction {:?}", node.instruction),
                }
            }
        }

        // NOTE: DO NOT INCLUDE STATIC INSTRUCTIONS or DAT!! Add them in the macro above
        label_to_addr!(ADD, SUB, STA, LDA, BRA, BRZ, BRP, BWA, BWO, BWX)
    }).collect())
}

pub fn assemble<S: AsRef<str>>(input: S) -> Result<Vec<Instruction<i64>>, ()> {
    let parser = parse();
    let ast = parser.parse(input.as_ref()).into_result().map_err(|_| ())?;
    resolve_labels(ast)
}

#[cfg(test)]
mod test {
    use super::*;
    use indoc::indoc;

    #[test]
    fn test_parse() {
        let test_doc = indoc! {"
            INP
        loop OUT
            STA count
            SUB one
            STA count
            BRP loop
            HLT

        one     DAT 1
        count   DAT
        "};
        println!("{}", test_doc);
        let parsed = assemble(test_doc).unwrap();
        println!("{:#?}", parsed);
    }
}
