// Copyright 2024, Giordano Salvador
// SPDX-License-Identifier: BSD-3-Clause

pub struct RunOptions {
    pub print_ast: bool,
    pub drop_token: bool,
    pub lex_exit: bool,
    pub parse_exit: bool,
    pub sem_exit: bool,
    pub verbose: bool,
}

impl RunOptions {
    pub fn new() -> Self {
        RunOptions{
            print_ast: false,
            drop_token: false,
            lex_exit: false,
            parse_exit: false,
            sem_exit: false,
            verbose: false,
        }
    }
}
