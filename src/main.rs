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
use std::io::Write;

mod ast;
mod exit_code;
mod irgen;
mod lex;
mod maingen;
mod module;
mod parse;
mod options;
mod sem;
mod target;

use ast::Ast;
use ast::Expr;
use exit_code::exit;
use exit_code::ExitCode;
use irgen::IRGen;
use lex::Lexer;
use lex::Token;
use maingen::MainGen;
use module::ModuleBundle;
use options::OptLevel;
use options::CodeGenType;
use options::RunOptions;
use parse::Parser;
use sem::Semantics;
use target::PassBuilder;
use target::Target;
use target::TargetMachine;

fn help(code: ExitCode) -> ! {
    eprintln!("usage: {} [OPTIONS] <INPUT>\n{}", PACKAGE, [
        "INPUT              '-' (i.e., Stdin) or a file path",
        "OPTIONS:",
        "--ast              Print the AST after parsing",
        "--drop             Drop unknown tokens instead of failing",
        "-e|--expr[=]<E>    Process expression E instead of INPUT file",
        "-h|--help          Print this list of command line options",
        "--lex              Exit after running the lexer",
        "-S|--llvmir        Exit after outputting LLVM IR (post-optimization) instead of byte code",
        "--nomain           Omit linking with main module (i.e., output kernel only)",
        "--notarget         Omit target specific configuration in LLVM IR/bytecode",
        "-o[=]<F>           Output to LLVM IR (.ll) or bytecode (.bc) instead of Stdout",
        "-O<0|1|2|3>        Set the optimization level (default: O2)",
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
    Stdout,
    File(&'a str),
}

fn output_type_to_string(t: OutputType) -> String {
    match t {
        OutputType::Stdout  => "Stdout".to_string(),
        OutputType::File(f) => format!("File:{}", f),
    }
}

fn parse_args<'a>(
    args: &'a Vec<String>,
    input: &mut InputType<'a>,
    output: &mut OutputType<'a>,
    options: &mut RunOptions
) {
    let _bin_name: &String = args.get(0).unwrap();
    let mut arg: &'a String;
    let mut i: usize = 1;

    while i < args.len() {
        arg = args.get(i).unwrap();
        match arg.as_str() {
            "--ast"         => options.print_ast = true,
            "--drop"        => options.drop_token = true,
            "-e"            => *input = InputType::Expr(parse_arg_after(args, &mut i)),
            "--expr"        => *input = InputType::Expr(parse_arg_after(args, &mut i)),
            "-h"            => help(ExitCode::Ok),
            "--help"        => help(ExitCode::Ok),
            "--lex"         => options.lex_exit = true,
            "--llvmir"      => options.codegen_type = CodeGenType::LLVMIR,
            "--nomain"      => options.no_main = true,
            "--notarget"    => options.no_target = true,
            "-o"            => *output = OutputType::File(parse_arg_after(args, &mut i)),
            "-O0"           => options.opt_level = OptLevel::O0,
            "-O1"           => options.opt_level = OptLevel::O1,
            "-O2"           => options.opt_level = OptLevel::O2,
            "-O3"           => options.opt_level = OptLevel::O3,
            "--parse"       => options.parse_exit = true,
            "--sem"         => options.sem_exit = true,
            "-S"            => options.codegen_type = CodeGenType::LLVMIR,
            "-v"            => options.verbose = true,
            "--verbose"     => options.verbose = true,
            "--version"     => print_pkg_info(true),
            _               => parse_arg_complex(arg, input, output),
        }
        i += 1;
    }

    if *input == InputType::None {
        eprintln!("No input file/name specified!");
        help(ExitCode::ArgParseError);
    } else if options.verbose {
        eprintln!("Processing input '{}'", input_type_to_string(*input));
    }

    match *output {
        OutputType::Stdout  => (),
        OutputType::File(f) => {
            let len = f.len();
            if len < 3 {
                eprintln!("Malformed output name '{}'", f);
                exit(ExitCode::ArgParseError);
            }
            let ext: &str = &f[len-3..];
            if ext != ".bc" && ext != ".ll" {
                eprintln!("Output name '{}' should end in '.bc' or '.ll", f);
                exit(ExitCode::ArgParseError);
            } else if ext == ".bc" && options.codegen_type == CodeGenType::LLVMIR {
                eprintln!("LLVM IR file type (with '-S' flag) should match output name ('.ll' extension)");
                exit(ExitCode::ArgParseError);
            } else if ext == ".ll" && options.codegen_type == CodeGenType::BYTECODE {
                eprintln!("Bytecode file type (missing '-S' flag) should match output name ('.bc' extension)");
                exit(ExitCode::ArgParseError);
            }
        },
    };

    if options.verbose {
        eprintln!("Outputting to '{}'", output_type_to_string(*output));
    }
}

fn parse_arg_after<'a>(args: &'a Vec<String>, i: &mut usize) -> &'a str {
    let name_option = args.get(*i).unwrap();
    match args.get(*i + 1) {
        Some(arg)   => {
            *i += 1;
            arg.as_str()
        },
        None        => {
            eprintln!("Expected argument after '{}' option", name_option);
            help(ExitCode::ArgParseError);
        },
    }
}

fn parse_arg_complex<'a>(
    arg: &'a String,
    input: &mut InputType<'a>,
    output: &mut OutputType<'a>,
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
                    "-o"        => *output = OutputType::File(&arg[j + 1..]),
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
    eprintln!("Welcome to {} version {}\n{}", PACKAGE, VERSION, LICENSE);
    if should_exit { exit(ExitCode::Ok); }
}

fn write_module(bundle: &mut ModuleBundle, codegen_type: CodeGenType, output: &OutputType) -> bool {
    match *output {
        OutputType::Stdout  => {
            match codegen_type {
                CodeGenType::LLVMIR => {
                    let string: String = bundle.to_string();
                    println!("{}", string);
                },
                CodeGenType::BYTECODE => {
                    eprintln!("Unimplemented: writing bytecode to Stdout");
                    return false;
                },
            }
        },
        OutputType::File(f) => {
            match codegen_type {
                CodeGenType::LLVMIR => {
                    let string: String = bundle.to_string();
                    let mut file = match File::create(f) {
                        Ok(file)    => file,
                        Err(msg)    => {
                            eprintln!("Failed to open output file '{}': {}", f, msg);
                            return false;
                        }
                    };
                    match file.write_all(string.as_bytes()) {
                        Ok(())      => (),
                        Err(msg)    => {
                            eprintln!("Failed to write to output file '{}': {}", f, msg);
                            return false;
                        }
                    }
                },
                CodeGenType::BYTECODE => bundle.write_bitcode_to_file(f),
            }
        },
    }

    return true;
}

fn main() -> ! {
    let args: Vec<String> = env::args().collect();
    let mut input: InputType = InputType::None;
    let mut output: OutputType = OutputType::Stdout;
    let mut options: RunOptions = RunOptions::new();

    parse_args(&args, &mut input, &mut output, &mut options);

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

    let module_name_irgen = String::from("calcc");
    let mut module_irgen = ModuleBundle::new(&module_name_irgen, options.verbose);
    let irgen_status: bool = IRGen::gen(*ast, &mut module_irgen, &options);
    assert!(irgen_status);
    let irgen_verify: bool = module_irgen.verify_module();
    if !irgen_verify {
        eprintln!("IRGen module failed to verify");
        exit(ExitCode::VerifyError);
    }

    let module_name_main = String::from("main");
    let mut module_main = if options.no_main {
        module_irgen
    } else {
        let mut module_main = ModuleBundle::new(&module_name_main, options.verbose);

        // NOTE: Fixing the verifier errors by reconstructing the callee signature using the correct main module
        // context causes the link step to omit the body of the callee (emitting only a declaration).
        //let f_sig = module_main.get_f_sig_from_context(&module_irgen.f_sig.clone().unwrap());
        let f_sig = &module_irgen.f_sig.clone().unwrap();

        let maingen_status: bool = MainGen::gen(&mut module_main, &f_sig, &options);
        assert!(maingen_status);

        // NOTE: Linking modules causes verifier errors for unmatched contexts of LLVMTypeRef. Skip verification.
        let link_status: bool = module_main.link_into(&mut module_irgen);
        if !link_status {
            eprintln!("IRGen and MainGen modules failed to link");
            exit(ExitCode::LinkError);
        }
        module_main
    };

    let mut target = Target::new();
    if options.verbose {
        eprintln!("Detected target triple '{}'", target.get_string());
    }
    let mut machine = TargetMachine::new(&mut target, options.opt_level);
    let mut pass_builder = PassBuilder::new();
    let opt_result = pass_builder.run(&mut module_main, &mut machine, options.opt_level, options.no_target);
    if !opt_result {
        exit(ExitCode::TargetError);
    }

    let write_result: bool = write_module(&mut module_main, options.codegen_type, &output);
    if !write_result {
        eprintln!("Failed to write module to output");
        exit(ExitCode::WriteError);
    }

    exit(ExitCode::Ok);
}
