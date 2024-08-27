// Copyright 2024, Giordano Salvador
// SPDX-License-Identifier: BSD-3-Clause
const LICENSE: &str = "\
// Copyright 2024, Giordano Salvador    \n\
// SPDX-License-Identifier: BSD-3-Clause\n\
";
const PACKAGE: &str = "calcc";
const VERSION: &str = "v0.1.0-dev";

use std::env;
use std::fs::File;
use std::io::Cursor;
use std::io::stdin;

mod ast;
mod exit_code;
mod irgen;
mod lex;
mod parse;
mod options;
mod sem;

use ast::Ast;
use ast::Expr;
use exit_code::exit;
use exit_code::ExitCode;
use irgen::ModuleBundle;
use irgen::IRGen;
use lex::Lexer;
use lex::Token;
use options::OptLevel;
use options::RunOptions;
use parse::Parser;
use sem::Semantics;

fn help(code: ExitCode) {
    println!("usage: {} [OPTIONS] <INPUT>\n{}", PACKAGE, [
        "INPUT              '-' (i.e., Stdin) or a file path",
        "OPTIONS:",
        "--ast              Print the AST after parsing",
        "--drop             Drop unknown tokens instead of failing",
        "-e|--expr[=]<E>    Process expression E instead of INPUT file",
        "-h|--help          Print this list of command line options",
        "--lex              Exit after running the lexer",
        "--llvmir           Exit after printing the llvmir",
        "-O<0,1,2,3,g>      Set the optimization level",
        "--parse            Exit after running the parser",
        "--sem              Exit after running the semantics check",
        "-v|--verbose       Enable verbose output",
        "--version          Display the package version and license information",
    ].join("\n"));
    exit(code);
}

#[derive(Clone,Copy,PartialEq)]
enum InputType<'a> {
    None,
    Stdin,
    Expr(&'a str),
    File(&'a str),
}

fn input_type_to_string(t: InputType) -> String {
    match t {
        InputType::Stdin    => "Stdin".to_string(),
        InputType::Expr(e)  => format!("Expression:{}", e),
        InputType::File(f)  => format!("File:{}", f),
        InputType::None     => "None".to_string(),
    }
}

#[derive(Clone,Copy)]
enum OutputType<'a> {
    None,
    Stdout,
    File(&'a str),
}

fn output_type_to_string(t: OutputType) -> String {
    match t {
        OutputType::Stdout  => "Stdout".to_string(),
        OutputType::File(f) => format!("File:{}", f),
        OutputType::None    => "None".to_string(),
    }
}

fn parse_args<'a>(args: &'a Vec<String>, input: &mut InputType<'a>, options: &mut RunOptions) {
    let _bin_name: &String = args.get(0).unwrap();
    let mut arg: &'a String;
    let mut i: usize = 1;

    while i < args.len() {
        arg = args.get(i).unwrap();
        match arg.as_str() {
            "--ast"     => options.print_ast = true,
            "--drop"    => options.drop_token = true,
            "-e"        => parse_arg_expr(args, &mut i, input),
            "--expr"    => parse_arg_expr(args, &mut i, input),
            "-h"        => help(ExitCode::Ok),
            "--help"    => help(ExitCode::Ok),
            "--lex"     => options.lex_exit = true,
            "--llvmir"  => options.print_ir = true,
            "-O0"       => options.opt_level = OptLevel::O0,
            "-O1"       => options.opt_level = OptLevel::O1,
            "-O2"       => options.opt_level = OptLevel::O2,
            "-O3"       => options.opt_level = OptLevel::O3,
            "-Og"       => options.opt_level = OptLevel::Og,
            "--parse"   => options.parse_exit = true,
            "--sem"     => options.sem_exit = true,
            "-v"        => options.verbose = true,
            "--verbose" => options.verbose = true,
            "--version" => print_pkg_info(true),
            _           => parse_arg_complex(arg, input),
        }
        i += 1;
    }

    if *input == InputType::None {
        eprintln!("No input file/name specified!");
        help(ExitCode::ArgParseError);
    } else if options.verbose {
        println!("Processing input '{}'", input_type_to_string(*input));
    }
}

fn parse_arg_expr<'a>(args: &'a Vec<String>, i: &mut usize, input: &mut InputType<'a>) {
    match args.get(*i + 1) {
        Some(arg)   => {
            *input = InputType::Expr(arg);
            *i += 1;
        }
        None        => {
            eprintln!("Expected expression argument after '-e|--expr' option");
            help(ExitCode::ArgParseError);
        }
    }
}

fn parse_arg_complex<'a>(
    arg: &'a String,
    input: &mut InputType<'a>,
) {
    let lead_char: char = arg.chars().next().unwrap();
    if arg.len() > 1 && lead_char == '-' {
        match arg.find('=') {
            None    => {
                eprintln!("Unrecognized argument '{}'", arg);
                help(ExitCode::ArgParseError);
            }
            Some(j) => {
                match &arg[0..j] {
                    "-e"        => *input = InputType::Expr(&arg[j + 1..]),
                    "--expr"    => *input = InputType::Expr(&arg[j + 1..]),
                    _           => {
                        eprintln!("Unrecognized argument '{}'", arg);
                        help(ExitCode::ArgParseError);
                    }
                }
            }
        }
    } else {
        if *input != InputType::None {
            eprintln!("Found more than one input ('{}' and '{}')", input_type_to_string(*input), arg);
            help(ExitCode::ArgParseError);
        } else if arg.len() == 1 && lead_char == '-' {
            *input = InputType::Stdin;
        } else {
            *input = InputType::File(arg.as_str());
        }
    }
}

fn print_pkg_info(should_exit: bool) {
    println!("Welcome to {} version {}\n{}", PACKAGE, VERSION, LICENSE);
    if should_exit { exit(ExitCode::Ok); }
}

fn main() -> ! {
    let args: Vec<String> = env::args().collect();
    let mut input: InputType = InputType::None;
    let mut options: RunOptions = RunOptions::new();

    parse_args(&args, &mut input, &mut options);

    let mut tokens: Vec<Token> = Vec::new();
    match input {
        InputType::None     => {
            eprintln!("Unexpected input");
            exit(ExitCode::ArgParseError);
        }
        InputType::Stdin    => {
            let mut lex = Lexer::new(stdin(), &options);
            Lexer::lex_input(&mut tokens, &mut lex, &options);
        }
        InputType::Expr(e)  => {
            let mut lex = Lexer::new(Cursor::new(e.to_string()), &options);
            Lexer::lex_input(&mut tokens, &mut lex, &options);
        }
        InputType::File(f)  => {
            let file: File = File::open(f).expect("Failed to open input file");
            let mut lex = Lexer::new(file, &options);
            Lexer::lex_input(&mut tokens, &mut lex, &options);
        }
    }

    let mut expr_tmp: Expr = Default::default();
    let mut ast: Box<&mut dyn Ast> = Box::new(&mut expr_tmp);
    let mut parser: Parser = Parser::new(&tokens, &options);
    Parser::parse_input(&mut ast, &mut parser, &options);

    let sem_check: bool = Semantics::check_all(*ast, &options);
    assert!(sem_check);

    let module_name = String::from("calcc");
    let mut module_bundle = ModuleBundle::new(&module_name);
    let irgen_status: bool = IRGen::gen(*ast, &mut module_bundle, &options);
    assert!(irgen_status);

    exit(ExitCode::Ok);
}
