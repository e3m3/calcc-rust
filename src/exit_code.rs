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
    _MainGenError = 7,
	VerifyError	= 8,
	TargetError	= 9,
	LinkError = 10,
	WriteError = 11,
}

pub fn exit(code: ExitCode) -> ! {
    process::exit(code as i32);
}
