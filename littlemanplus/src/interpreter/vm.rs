use lmp_common::{MEMORY_SIZE, assembly};
use lmp_lang::parser;
use thiserror::Error;

#[derive(Debug)]
pub struct VirtualMachine {
    /// Points to a location in memory that the virtual machine is currently at
    program_counter: usize,
    accumulator: i64,
    /// Represents the memory of the virtual machine
    memory: [MemoryCell; MEMORY_SIZE],

    // Debug information
    cycles: i64,
}

impl VirtualMachine {
    pub fn new() -> Self {
        Self {
            program_counter: 0,
            accumulator: 0,
            memory: [MemoryCell::default(); MEMORY_SIZE],
            cycles: 0
        }
    }

    /// Compile, load and run a provided assembly program
    pub fn compile_run<S: AsRef<str>>(&mut self, program: S) -> Result<(), VirtualMachineError> {
        let Ok(compiled) = parser::assemble(program.as_ref().trim()) else {
            return Err(VirtualMachineError::CompilerError);
        };

        if compiled.len() > MEMORY_SIZE {
            return Err(VirtualMachineError::MemoryFull);
        }

        // Load program into memory instruction by instruction
        let mut counter = 0usize;
        // First pass: Load all DAT instructions
        for instr in &compiled {
            if let assembly::Instruction::DAT(data) = instr {
                self.write(counter, *data);
            }

            counter += 1;
        }
        // Reset counter for the second pass
        counter = 0;
        // Second pass: write instructions
        for instr in compiled {
            self.write(counter, instr.into());

            counter += 1;
        }

        // Run the program
        loop {
            self.cycles += 1;

            if self.program_counter >= MEMORY_SIZE {
                println!("program ran to end of memory");
                break;
            }

            // Fetch
            let cell = self.memory[self.program_counter];
            // Decode
            let Ok(decoded) = cell.data.try_into() else {
                println!("failed to decode instruction");
                self.program_counter += 1;
                continue;
            };
            // Execute
            use assembly::Instruction::*;
            match decoded {
                ADD(addr) => {
                    let referenced_cell = self.ptr_get(addr);
                    self.accumulator += referenced_cell.data;
                    self.program_counter += 1
                }
                SUB(addr) => {
                    let referenced_cell = self.ptr_get(addr);
                    self.accumulator -= referenced_cell.data;
                    self.program_counter += 1
                },
                STA(addr) => {
                    self.ptr_write(addr, self.accumulator);
                    self.program_counter += 1
                },
                LDA(addr) => {
                    let referenced_cell = self.ptr_get(addr);
                    self.accumulator = referenced_cell.data;
                    self.program_counter += 1
                },
                BRA(addr) => {
                    self.branch(addr);
                },
                BRZ(addr) => {
                    if self.accumulator == 0 {
                        self.branch(addr);
                    } else {
                        self.program_counter += 1;
                    }
                }
                BRP(addr) => {
                    // BRP includes zero based on 101computing's LMC
                    if self.accumulator >= 0 {
                        self.branch(addr);
                    } else {
                        self.program_counter += 1;
                    }
                },
                BWN => {
                    self.accumulator = !self.accumulator;
                    self.program_counter += 1;
                }
                BWA(addr) => {
                    let referenced_cell = self.ptr_get(addr);
                    self.accumulator = self.accumulator & referenced_cell.data;
                    self.program_counter += 1;
                },
                BWO(addr) => {
                    let referenced_cell = self.ptr_get(addr);
                    self.accumulator = self.accumulator | referenced_cell.data;
                    self.program_counter += 1;
                }
                BWX(addr) => {
                    let referenced_cell = self.ptr_get(addr);
                    self.accumulator = self.accumulator ^ referenced_cell.data;
                    self.program_counter += 1;
                }
                INP => {
                    // TODO: ask user for input
                    self.accumulator = 10;
                    self.program_counter += 1;
                },
                OUT => {
                    println!("OUTPUT: {}", self.accumulator);
                    self.program_counter += 1;
                },
                HLT => break,
                DAT(_) => unreachable!("DAT instruction should have been removed by the compiler"),
            }
        }

        println!("Program executed in {} cycles", self.cycles);

        Ok(())
    }

    /// Set the `program_counter` to a value (usually obtained from memory)
    ///
    /// Checks if the provided new `ptr` can fit into a `usize` (so it can be used to index the `memory` array)
    /// and also checks if it fits within [`MEMORY_SIZE`].
    fn branch(&mut self, ptr: i64) {
        let ptr: usize = ptr.try_into().unwrap();

        if ptr >= MEMORY_SIZE {
            panic!("pointer out of bounds");
        }

        self.program_counter = ptr;
    }

    /// Write to a location in memory
    fn write(&mut self, loc: usize, data: i64) {
        if loc >= MEMORY_SIZE {
            panic!("write loc out of bounds at {loc}")
        }

        self.memory[loc].set(data);
    }

    /// Write to a cell in memory with an [`i64`] pointer
    fn ptr_write(&mut self, ptr: i64, data: i64) {
        let loc = self.ptr_to_loc(ptr);

        self.write(loc, data);
    }

    /// Get the cell occupying a location in memory from an [`i64`], following pointers
    ///
    /// # Panics
    /// If the [`i64`] cannot be converted into a [`usize`] or is out of bounds
    fn ptr_get(&self, ptr: i64) -> MemoryCell {
        let loc = self.ptr_to_loc(ptr);

        self.memory[loc]
    }

    /// Resolve the actual [`usize`] memory address of a specified [`i64`]
    /// following pointers as needed
    ///
    /// # Panics
    /// If the [`i64`] cannot be converted into a [`usize`] or is out of bounds
    fn ptr_to_loc(&self, ptr: i64) -> usize {
        let loc = ptr as usize;

        if loc >= MEMORY_SIZE * 2 {
            panic!("ptr {ptr} out of bounds and was not valid pointer")
        }

        // Pointer (MEMORY_SIZE + LOCATION) indicates a pointer at that location
        if loc > MEMORY_SIZE {
            // Follow the pointer recursively
            self.ptr_to_loc(self.memory[loc - MEMORY_SIZE].data)
        } else {
            // Or else, return the converted loc
            loc
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct MemoryCell {
    pub data: i64,
}

impl Default for MemoryCell {
    fn default() -> Self {
        Self { data: 0 }
    }
}

impl MemoryCell {
    /// Set the data of this cell
    fn set(&mut self, data: i64) {
        self.data = data
    }
}

#[derive(Debug, Error)]
pub enum VirtualMachineError {
    #[error("Could not compile the program. Check for errors in assembly code.")]
    CompilerError,
    #[error(
        "The program is too big to fit into the memory. Program can be a maximum of {} instructions long",
        MEMORY_SIZE
    )]
    MemoryFull,
}
