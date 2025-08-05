use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DataOrLabel<'a, Data> {
    Data(Data),
    Label(&'a str),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i64)]
pub enum Instruction<Data> {
    /// Add the contents of the memory at the specified address to the accumulator
    ADD(Data) = 1000,
    /// Subtract the contents of the memory at the specified address to the accumulator
    SUB(Data) = 2000,
    /// Store the contents of the accumulator at the specified memory address, overwriting
    STA(Data) = 3000,
    /// Load the contents of the specified memory address into the accumulator, overwriting
    LDA(Data) = 5000,
    /// Branch always: set the program counter to the specified memory address
    BRA(Data) = 6000,
    /// Branch if zero, sets the program counter to the specified memory address if the accumulator is
    /// zero
    BRZ(Data) = 7000,
    /// Branch if positive, sets the program counter to the specified memory address if the \
    /// accumulator is positive
    BRP(Data) = 8000,

    // Bitwise operations
    /// Bitwise NOT the accumulator
    BWN = 10000,
    /// Bitwise AND the accumulator with the specified memory address
    BWA(Data) = 11000,
    /// Bitwise OR the accumulator with the specified memory address
    BWO(Data) = 12000,
    /// Bitwise XOR the accumulator with the specified memory address
    BWX(Data) = 13000,

    /// Request input from the user which is stored into the accumulator, overwriting
    INP = 901,
    /// Output the value currently in the accumulator, does not overwrite
    OUT = 902,
    /// Stop the program
    HLT = 1,
    /// Store a piece of data at a free memory address, usually associating it with a label.
    ///
    /// Data defaults to `0`
    DAT(Data) = 0,
}

impl Into<i64> for Instruction<i64> {
    fn into(self) -> i64 {
        use Instruction::*;
        match self {
            HLT => 1,
            INP => 901,
            OUT => 902,
            BWN => 10000,

            ADD(addr) => 1000 + addr,
            SUB(addr) => 2000 + addr,
            STA(addr) => 3000 + addr,
            LDA(addr) => 5000 + addr,
            BRA(addr) => 6000 + addr,
            BRZ(addr) => 7000 + addr,
            BRP(addr) => 8000 + addr,

            BWA(addr) => 11000 + addr,
            BWO(addr) => 12000 + addr,
            BWX(addr) => 13000 + addr,

            // Not really an instruction, return the data
            DAT(data) => data,
        }
    }
}

impl TryFrom<i64> for Instruction<i64> {
    type Error = ();

    /// Decode instruction from raw [i64]. Does NOT work for pseudo-instruction [`Instruction::DAT`]
    fn try_from(value: i64) -> Result<Self, Self::Error> {
        use Instruction::*;
        match value {
            // Fixed instructions
            1 => Ok(HLT),
            901 => Ok(INP),
            902 => Ok(OUT),
            10000 => Ok(BWN),
            // Dynamic instructions
            1000..=1999 => Ok(ADD(value - 1000)),
            2000..=2999 => Ok(SUB(value - 2000)),
            3000..=3999 => Ok(STA(value - 3000)),
            5000..=5999 => Ok(LDA(value - 5000)),
            6000..=6999 => Ok(BRA(value - 6000)),
            7000..=7999 => Ok(BRZ(value - 7000)),
            8000..=8999 => Ok(BRP(value - 8000)),
            11000..=11999 => Ok(BWA(value - 11000)),
            12000..=12999 => Ok(BWO(value - 12000)),
            13000..=13999 => Ok(BWX(value - 13000)),
            _ => Err(()),
        }
    }
}

impl<Data: fmt::Display> fmt::Display for Instruction<Data> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Instruction::*;
        match self {
            INP => write!(f, "INP"),
            OUT => write!(f, "OUT"),
            HLT => write!(f, "HLT"),

            ADD(loc) => write!(f, "ADD {}", loc),
            SUB(loc) => write!(f, "SUB {}", loc),

            STA(loc) => write!(f, "STA {}", loc),
            LDA(loc) => write!(f, "LDA {}", loc),

            BRA(loc) => write!(f, "BRA {}", loc),
            BRZ(loc) => write!(f, "BRZ {}", loc),
            BRP(loc) => write!(f, "BRP {}", loc),

            BWN => write!(f, "BWN"),
            BWA(loc) => write!(f, "BWA {}", loc),
            BWO(loc) => write!(f, "BWO {}", loc),
            BWX(loc) => write!(f, "BWX {}", loc),

            DAT(loc) => write!(f, "DAT {}", loc),
        }
    }
}
