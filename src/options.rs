// Copyright 2024, Giordano Salvador
// SPDX-License-Identifier: BSD-3-Clause

pub struct RunOptions {
    pub drop_token: bool,
    pub lex_exit: bool,
    pub opt_level: OptLevel,
    pub parse_exit: bool,
    pub print_ast: bool,
    pub print_ir: bool,
    pub sem_exit: bool,
    pub verbose: bool,
}

impl RunOptions {
    pub fn new() -> Self {
        RunOptions{
            drop_token: false,
            lex_exit: false,
            opt_level: OptLevel::O0,
            parse_exit: false,
            print_ast: false,
            print_ir: false,
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
    O2 = 2,
    O3 = 3,
    Og = 4,
}
