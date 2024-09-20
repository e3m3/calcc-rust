// Copyright 2024, Giordano Salvador
// SPDX-License-Identifier: BSD-3-Clause

use std::process;

#[repr(u8)]
#[derive(Clone,Copy)]
pub enum ExitCode {
    Ok = 0,
    ArgParseError = 1,
    LexerError = 2,
    ParserError = 3,
    SemanticError = 4,
    ModuleError = 5,
    IRGenError = 6,
    MainGenError = 7,
    MainGenCError = 8,
    VerifyError = 9,
    TargetError = 10,
    LinkError = 11,
    WriteError = 12,
    CommandError = 13,
}

pub fn exit(code: ExitCode) -> ! {
    process::exit(code as i32);
}
