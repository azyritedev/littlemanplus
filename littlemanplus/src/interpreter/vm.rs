use lmp_common::{assembly, MEMORY_SIZE};
use lmp_lang::parser;
use std::fmt::Debug;
use thiserror::Error;

#[derive(Debug)]
pub struct VirtualMachine {
    /// Points to a location in memory that the virtual machine is currently at
    program_counter: usize,
    accumulator: i64,
    /// Represents the memory of the virtual machine
    memory: [MemoryCell; MEMORY_SIZE],
    /// Whether the virtual machine has reached a halt condition
    halted: bool,

    /// I/O
    input_buffer: Option<i64>,

    // Debug information
    cycles: i64,
    /// Last accessed memory location
    accessing: usize,
}

impl VirtualMachine {
    pub fn new() -> Self {
        Self {
            program_counter: 0,
            accumulator: 0,
            memory: [MemoryCell::default(); MEMORY_SIZE],
            cycles: 0,
            accessing: 0,
            halted: false,
            input_buffer: None,
        }
    }

    /// Compile the provided assembly program and load it into the virtual machine's memory
    pub fn compile<S: AsRef<str>>(&mut self, program: S) -> Result<(), VirtualMachineError> {
        let Ok(compiled) = parser::assemble(program.as_ref().trim()) else {
            return Err(VirtualMachineError::CompilerError);
        };

        if compiled.len() > MEMORY_SIZE {
            return Err(VirtualMachineError::MemoryFull);
        }

        // Clear memory and reset registers
        self.reset();

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

        // Reset halt state
        self.halted = false;

        Ok(())
    }

    /// Reset the state of the VM (does nothing if VM is not halted)
    pub fn reset(&mut self) {
        if !self.halted { return; }

        self.memory = [MemoryCell::default(); MEMORY_SIZE];
        self.cycles = 0;
        self.accessing = 0;
        self.accumulator = 0;
        self.program_counter = 0;
        self.input_buffer = None;
    }

    pub fn step(&mut self) -> VirtualMachineStep {
        if self.halted {
            return VirtualMachineStep::Halted;
        }

        self.cycles += 1;

        if self.program_counter >= MEMORY_SIZE {
            self.halted = true;
            return VirtualMachineStep::Halted;
        }

        // Fetch
        let cell = self.memory[self.program_counter];
        // Decode
        let Ok(decoded) = cell.data.try_into() else {
            self.program_counter += 1;
            // Skip the undecodable instruction; might change to halting the VM in the future
            return VirtualMachineStep::Advanced;
        };
        // Execute
        use assembly::Instruction::*;
        match decoded {
            ADD(addr) => {
                let referenced_cell = self.ptr_get(addr);
                self.accumulator += referenced_cell.data;
                self.program_counter += 1;
                VirtualMachineStep::Advanced
            }
            SUB(addr) => {
                let referenced_cell = self.ptr_get(addr);
                self.accumulator -= referenced_cell.data;
                self.program_counter += 1;
                VirtualMachineStep::Advanced
            },
            STA(addr) => {
                self.ptr_write(addr, self.accumulator);
                self.program_counter += 1;
                VirtualMachineStep::Advanced
            },
            LDA(addr) => {
                let referenced_cell = self.ptr_get(addr);
                self.accumulator = referenced_cell.data;
                self.program_counter += 1;
                VirtualMachineStep::Advanced
            },
            BRA(addr) => {
                self.branch(addr);
                VirtualMachineStep::Advanced
            },
            BRZ(addr) => {
                if self.accumulator == 0 {
                    self.branch(addr);
                } else {
                    self.program_counter += 1;
                }

                VirtualMachineStep::Advanced
            }
            BRP(addr) => {
                // BRP includes zero based on 101computing's LMC
                if self.accumulator >= 0 {
                    self.branch(addr);
                } else {
                    self.program_counter += 1;
                }

                VirtualMachineStep::Advanced
            },
            BWN => {
                self.accumulator = !self.accumulator;
                self.program_counter += 1;
                VirtualMachineStep::Advanced
            }
            BWA(addr) => {
                let referenced_cell = self.ptr_get(addr);
                self.accumulator = self.accumulator & referenced_cell.data;
                self.program_counter += 1;
                VirtualMachineStep::Advanced
            },
            BWO(addr) => {
                let referenced_cell = self.ptr_get(addr);
                self.accumulator = self.accumulator | referenced_cell.data;
                self.program_counter += 1;
                VirtualMachineStep::Advanced
            }
            BWX(addr) => {
                let referenced_cell = self.ptr_get(addr);
                self.accumulator = self.accumulator ^ referenced_cell.data;
                self.program_counter += 1;
                VirtualMachineStep::Advanced
            }
            LDR => {
                let referenced_cell = self.ptr_get(self.accumulator);
                self.accumulator = referenced_cell.data;
                self.program_counter += 1;
                VirtualMachineStep::Advanced
            }
            INP => {
                if let Some(input) = self.input_buffer.take() {
                    // Attempt to take the input buffer, if it has a value, use it
                    self.accumulator = input;
                    self.program_counter += 1;
                    VirtualMachineStep::Advanced
                } else {
                    // Else do not step and ask for input
                    VirtualMachineStep::InputRequired
                }
            },
            OUT => {
                self.program_counter += 1;
                VirtualMachineStep::Output(self.accumulator)
            },
            HLT => {
                self.halted = true;
                VirtualMachineStep::Halted
            },
            DAT(_) => unreachable!("DAT instruction should have been removed by the compiler"),
        }
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
    fn ptr_get(&mut self, ptr: i64) -> MemoryCell {
        let loc = self.ptr_to_loc(ptr);

        self.memory[loc]
    }

    /// Resolve the actual [`usize`] memory address of a specified [`i64`]
    /// following pointers as needed
    ///
    /// Also records memory location accessed (so it requires mutability)
    ///
    /// # Panics
    /// If the [`i64`] cannot be converted into a [`usize`] or is out of bounds
    fn ptr_to_loc(&mut self, ptr: i64) -> usize {
        let loc = ptr as usize;

        if loc >= MEMORY_SIZE * 2 {
            panic!("ptr {ptr} out of bounds and was not valid pointer")
        }

        // Pointer (MEMORY_SIZE + LOCATION) indicates a pointer at that location
        if loc > MEMORY_SIZE {
            // Follow the pointer recursively
            let resolved_loc = loc - MEMORY_SIZE;
            self.accessing = resolved_loc;
            self.ptr_to_loc(self.memory[resolved_loc].data)
        } else {
            // Or else, return the converted loc
            self.accessing = loc;
            loc
        }
    }

    // Public access methods
    pub fn accumulator(&self) -> i64 {
        self.accumulator
    }

    pub fn program_counter(&self) -> usize {
        self.program_counter
    }

    pub fn cycles(&self) -> i64 {
        self.cycles
    }

    pub fn halted(&self) -> bool {
        self.halted
    }

    pub fn input(&mut self, input: i64) {
        self.input_buffer = Some(input);
    }

    pub fn memory(&self) -> &[MemoryCell] {
        &self.memory
    }

    pub fn accessing(&self) -> usize {
        self.accessing
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

/// The result of the VM after stepping it by one cycle
#[derive(Debug)]
pub enum VirtualMachineStep {
    /// The VM has executed an instruction
    Advanced,
    /// The VM produced an output
    Output(i64),
    /// The VM requires an input and will block here without advancing the program counter until the
    /// input buffer is filled
    InputRequired,
    /// The VM has reached a halt condition
    Halted,
}