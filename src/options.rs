// Copyright 2024, Giordano Salvador
// SPDX-License-Identifier: BSD-3-Clause

pub struct RunOptions {
    pub codegen_type: CodeGenType,
    pub drop_token: bool,
    pub lex_exit: bool,
    pub no_main: bool,
    pub no_target: bool,
    pub opt_level: OptLevel,
    pub parse_exit: bool,
    pub print_ast: bool,
    pub sem_exit: bool,
    pub verbose: bool,
}

impl RunOptions {
    pub fn new() -> Self {
        RunOptions{
            codegen_type: CodeGenType::Bytecode,
            drop_token: false,
            lex_exit: false,
            no_main: false,
            no_target: false,
            opt_level: OptLevel::O2,
            parse_exit: false,
            print_ast: false,
            sem_exit: false,
            verbose: false,
        }
    }
}

#[repr(u8)]
#[derive(Clone,Copy)]
pub enum OptLevel {
    O0 = 0,
    O1 = 1,
    O2 = 2, /// LLVM default opt level
    O3 = 3,
}

pub fn opt_level_to_str(opt_level: OptLevel) -> String {
    String::from(match opt_level {
        OptLevel::O0    => "O0",
        OptLevel::O1    => "O1",
        OptLevel::O2    => "O2",
        OptLevel::O3    => "O3",
    })
}

#[repr(u8)]
#[derive(Clone,Copy,PartialEq)]
pub enum CodeGenType {
    Llvmir      = 0,
    Bytecode    = 1,
}
