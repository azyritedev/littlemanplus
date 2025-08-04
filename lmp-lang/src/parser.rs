//! Assembly compiler

use chumsky::prelude::*;
use lmp_common::assembly::Instruction;
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
    text::ascii::ident()
        // HACK: Use rewind?
        // If all uppercase then reject as it is probably an opcode (e.g., ADD),
        // this prevents label from "eating" the opcode when no label is provided
        .try_map(|s: &str, _span| {
            if s.chars().all(|c| c.is_ascii_uppercase()) {
                Err(EmptyErr::default())
            } else {
                Ok(s)
            }
        })
}

/// whitespace excl. newlines
fn whitespace<'a>() -> impl Parser<'a, &'a str, ()> {
    text::inline_whitespace().at_least(1)
}

fn instruction<'a>() -> impl Parser<'a, &'a str, NodeInstruction<'a>> {
    // Allow any indent
    text::inline_whitespace().ignore_then(choice((
        just("INP").to(Instruction::INP),
        just("OUT").to(Instruction::OUT),
        just("HLT").to(Instruction::HLT),
        just("DAT").ignore_then(
            whitespace()
                .ignore_then(num())
                .or_not()
                .map(|data| Instruction::DAT(NodeInstructionData::Num(data.unwrap_or_default()))),
        ),
        just("ADD")
            .ignore_then(whitespace().ignore_then(text::ascii::ident()))
            .map(|label| Instruction::ADD(label.into())),
        just("SUB")
            .ignore_then(whitespace().ignore_then(text::ascii::ident()))
            .map(|label| Instruction::SUB(label.into())),
        just("STA")
            .ignore_then(whitespace().ignore_then(text::ascii::ident()))
            .map(|label| Instruction::STA(label.into())),
        just("LDA")
            .ignore_then(whitespace().ignore_then(text::ascii::ident()))
            .map(|label| Instruction::LDA(label.into())),
        just("BRA")
            .ignore_then(whitespace().ignore_then(text::ascii::ident()))
            .map(|label| Instruction::BRA(label.into())),
        just("BRZ")
            .ignore_then(whitespace().ignore_then(text::ascii::ident()))
            .map(|label| Instruction::BRZ(label.into())),
        just("BRP")
            .ignore_then(whitespace().ignore_then(text::ascii::ident()))
            .map(|label| Instruction::BRP(label.into())),
    )))
}

fn labeled_line<'a>() -> impl Parser<'a, &'a str, Node<'a>> {
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
                    $member(NodeInstructionData::Label(l)) => {
                        let addr = *labels.get(l).expect(&format!("invalid label {l}"));
                        $member(addr as i64)
                    }
                    // TODO: Pointers
                    // $member(NodeInstructionData::Pointer(p)) => {
                    //     let
                    // }
                )*
                    DAT(NodeInstructionData::Num(n)) => DAT(n),
                    INP => INP,
                    OUT => OUT,
                    HLT => HLT,
                    _ => panic!("invalid instruction"),
                }
            }
        }

        label_to_addr!(ADD, SUB, STA, LDA, BRA, BRZ, BRP)
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
