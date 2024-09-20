// Copyright 2024, Giordano Salvador
// SPDX-License-Identifier: BSD-3-Clause

use std::fmt;

pub struct RunOptions {
    pub body_type: BodyType,
    pub codegen_type: CodeGenType,
    pub drop_token: bool,
    pub host_arch: HostArch,
    pub host_os: HostOS,
    pub ir_exit: bool,
    pub lex_exit: bool,
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
            body_type: BodyType::Unset,
            codegen_type: CodeGenType::Unset,
            drop_token: false,
            host_arch: get_host_arch(),
            host_os: get_host_os(),
            ir_exit: false,
            lex_exit: false,
            no_target: false,
            opt_level: OptLevel::O2,
            parse_exit: false,
            print_ast: false,
            sem_exit: false,
            verbose: false,
        }
    }

    pub fn early_exit(&self) -> bool {
        self.ir_exit || self.lex_exit || self.parse_exit || self.sem_exit
    }
}

impl fmt::Display for RunOptions {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s_vec = vec![
            "RunOptions:".to_string(),
            format!("body_type: {}",    self.body_type),
            format!("codegen_type: {}", self.codegen_type),
            format!("drop_token: {}",   self.drop_token),
            format!("host_arch: {}",    self.host_arch),
            format!("host_os: {}",      self.host_os),
            format!("ir_exit: {}",      self.ir_exit),
            format!("lex_exit: {}",     self.lex_exit),
            format!("no_target: {}",    self.no_target),
            format!("opt_level: {}",    self.opt_level),
            format!("parse_exit: {}",   self.parse_exit),
            format!("print_ast: {}",    self.print_ast),
            format!("sem_exit: {}",     self.sem_exit),
            format!("verbose: {}",      self.verbose),
        ];
        write!(f, "{}", s_vec.join("\n    "))
    }
}

#[derive(Clone,Copy)]
pub enum OutputType<'a> {
    Stdout,
    File(&'a str),
}

impl <'a> fmt::Display for OutputType<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            OutputType::Stdout  => "Stdout".to_string(),
            OutputType::File(f) => format!("File:{}", f),
        };
        write!(f, "{}", s)
    }
}

#[repr(u8)]
#[derive(Clone,Copy,Default)]
pub enum OptLevel {
    O0 = 0,
    O1 = 1,
    #[default]
    O2 = 2,     /// LLVM default opt level
    O3 = 3,
}

impl fmt::Display for OptLevel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            OptLevel::O0 => "OptLevel_O0",
            OptLevel::O1 => "OptLevel_O1",
            OptLevel::O2 => "OptLevel_O2",
            OptLevel::O3 => "OptLevel_O3",
        };
        write!(f, "{}", s)
    }
}

#[repr(u8)]
#[derive(Clone,Copy,Default,PartialEq)]
pub enum BodyType {
    #[default]
    Unset       = 0,
    MainGen     = 1,
    MainGenC    = 2,
    NoMain      = 3,
}

impl fmt::Display for BodyType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            BodyType::Unset     => "BodyType_Unset",
            BodyType::MainGen   => "BodyType_MainGen",
            BodyType::MainGenC  => "BodyType_MainGenC",
            BodyType::NoMain    => "BodyType_NoMain",
        };
        write!(f, "{}", s)
    }
}

#[repr(u8)]
#[derive(Clone,Copy,Default,PartialEq)]
pub enum CodeGenType {
    #[default]
    Unset       = 0,
    Llvmir      = 1,
    Bitcode     = 2,
    Object      = 3,
    Executable  = 4,
}

impl fmt::Display for CodeGenType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            CodeGenType::Unset      => "CodeGen_Unset",
            CodeGenType::Llvmir     => "CodeGen_Llvmir",
            CodeGenType::Bitcode    => "CodeGen_Bitcode",
            CodeGenType::Object     => "CodeGen_Object",
            CodeGenType::Executable => "CodeGen_Executable",
        };
        write!(f, "{}", s)
    }
}

#[repr(u8)]
#[derive(Clone,Copy,Default,PartialEq)]
pub enum HostArch {
    #[default]
    Unknown     = 0,
    Aarch64     = 1,
    X86         = 2,
    X86_64      = 3,
}

impl fmt::Display for HostArch {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            HostArch::Unknown   => "HostArch_Unknown",
            HostArch::Aarch64   => "HostArch_Aarch64",
            HostArch::X86       => "HostArch_X86",
            HostArch::X86_64    => "HostArch_X86_64",
        };
        write!(f, "{}", s)
    }
}

pub fn get_host_arch() -> HostArch {
    if cfg!(target_arch = "aarch64") {
        HostArch::Aarch64
    } else if cfg!(target_arch = "x86") {
        HostArch::X86
    } else if cfg!(target_arch = "x86_64") {
        HostArch::X86_64
    } else {
        HostArch::Unknown
    }
}

#[repr(u8)]
#[derive(Clone,Copy,Default,PartialEq)]
pub enum HostOS {
    #[default]
    Unknown     = 0,
    Linux       = 1,
    MacOS       = 2,
    Windows     = 3,
}

impl fmt::Display for HostOS {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            HostOS::Unknown => "HostOS_Unknown",
            HostOS::Linux   => "HostOS_Linux",
            HostOS::MacOS   => "HostOS_MacOS",
            HostOS::Windows => "HostOS_Windows",
        };
        write!(f, "{}", s)
    }
}

pub fn get_host_os() -> HostOS {
    if cfg!(target_os = "linux") {
        HostOS::Linux
    } else if cfg!(target_os = "macos") {
        HostOS::MacOS
    } else if cfg!(target_os = "windows") {
        HostOS::Windows
    } else {
        HostOS::Unknown
    }
}
