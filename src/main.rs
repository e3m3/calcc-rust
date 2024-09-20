// Copyright 2024, Giordano Salvador
// SPDX-License-Identifier: BSD-3-Clause
const LICENSE: &str = "\
// Copyright 2024, Giordano Salvador    \n\
// SPDX-License-Identifier: BSD-3-Clause\n\
";
const PACKAGE: &str = "calcc";
const VERSION: &str = "v0.2.0";

use std::env;
use std::fmt;
use std::fs::File;
use std::io::Cursor;
use std::io::stdin;
use std::path::Path;
use std::process;

mod ast;
mod command;
mod exit_code;
mod irgen;
mod lex;
mod maingen;
mod maingen_c;
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
use maingen_c::MainGenC;
use module::ModuleBundle;
use options::BodyType;
use options::CodeGenType;
use options::HostOS;
use options::OptLevel;
use options::OutputType;
use options::RunOptions;
use parse::Parser;
use sem::Semantics;
use target::Passes;
use target::PassBuilder;
use target::Target;
use target::TargetMachine;

fn help(code: ExitCode) -> ! {
    eprintln!("usage: {} [OPTIONS] <INPUT>\n{}", PACKAGE, [
        "INPUT              '-' (i.e., Stdin) or a file path",
        "OPTIONS:",
        "--ast              Print the AST after parsing",
        "-b|--bitcode       Output LLVM bitcode (post-optimization) (.bc if used with -o)",
        "-c                 Output an object file (post-optimization) (.o if used with -o)",
        "--drop             Drop unknown tokens instead of failing",
        "-e|--expr[=]<E>    Process expression E instead of INPUT file",
        "-h|--help          Print this list of command line options",
        "--lex              Exit after running the lexer",
        "--ir               Exit after printing IR (pre-optimization)",
        "-S|--llvmir        Output LLVM IR (post-optimization) (.ll if used with -o)",
        "-k|--no-main       Omit linking with main module (i.e., output kernel only)",
        "                   When this option is selected, an executable cannot be generated",
        "--notarget         Omit target specific configuration in LLVM IR/bitcode",
        "-o[=]<F>           Output to file F instead of Stdout",
        "                   If no known extension is used (.bc|.exe|.ll|.o) an executable is assumed",
        "                   An executable requires llc and clang to be installed",
        "-O<0|1|2|3>        Set the optimization level (default: O2)",
        "--parse            Exit after running the parser",
        "--sem              Exit after running the semantics check",
        "-C|--c-main        Link with a C-derived main module (src/main.c.template)",
        "                   This option is required for generating object files and executables on MacOS",
        "                   and requires clang to be installed",
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

impl <'a> fmt::Display for InputType<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            InputType::Stdin    => "Stdin".to_string(),
            InputType::Expr(e)  => format!("Expression:{}", e),
            InputType::File(f)  => format!("File:{}", f),
            InputType::None     => "None".to_string(),
        };
        write!(f, "{}", s)
    }
}

fn set_codegen_type(options: &mut RunOptions, codegen_type: CodeGenType) -> () {
    if options.codegen_type == CodeGenType::Unset {
        options.codegen_type = codegen_type;
    } else {
        eprintln!("Incompatible compiler flags for output type: {} and {}", options.codegen_type, codegen_type);
        exit(ExitCode::ArgParseError);
    }
}

fn set_body_type(options: &mut RunOptions, body_type: BodyType) -> () {
    if options.body_type == BodyType::Unset {
        options.body_type = body_type;
    } else {
        eprintln!("Incompatible compiler flags: '-k|--no-main' and '-C|--c-main'");
        exit(ExitCode::ArgParseError);
    }
}

#[derive(Clone,Copy)]
enum ExtType {
    None,
    BC,
    Exe,
    LL,
    O,
}

fn get_extension_from_filename(name: &str) -> ExtType {
    match Path::new(name).extension() {
        None        => ExtType::None,
        Some(ext)   => match ext.to_str().unwrap() {
            "bc"    => ExtType::BC,
            "exe"   => ExtType::Exe,
            "ll"    => ExtType::LL,
            "o"     => ExtType::O,
            _       => ExtType::None,
        },
    }
}

/// Checks to ensure valid combination for BodyType, CodeGenType, and OutputType
fn check_options_configuration(options: &RunOptions, output: &OutputType) -> () {
    match *output {
        OutputType::Stdout  => (),
        OutputType::File(f) => {
            let t = options.codegen_type;
            match get_extension_from_filename(f) {
                ExtType::None   => if t != CodeGenType::Executable {
                    eprintln!("Output name (no/unknown extension) should match codegen type: {} specified", t);
                    exit(ExitCode::ArgParseError);
                },
                ExtType::BC     => if t != CodeGenType::Bitcode {
                    eprintln!("Output name ('.bc' extension) should match codegen type (-b|--bitcode)");
                    exit(ExitCode::ArgParseError);
                },
                ExtType::Exe    => if t != CodeGenType::Executable {
                    eprintln!("Output name ('.exe' extension) should match codegen type: {} specified", t);
                    exit(ExitCode::ArgParseError);
                },
                ExtType::LL     => if t != CodeGenType::Llvmir {
                    eprintln!("Output name ('.ll' extension) should match codegen type (-S|--llvmir)");
                    exit(ExitCode::ArgParseError);
                },
                ExtType::O      => if t != CodeGenType::Object {
                    eprintln!("Output name ('.o' extension) should match codegen type (-c)");
                    exit(ExitCode::ArgParseError);
                },
            }
        },
    };

    // If early exit is enabled, don't worry about incompatible output types below
    if options.early_exit() {
        return;
    }

    if options.body_type == BodyType::NoMain && options.codegen_type == CodeGenType::Executable {
        eprintln!("Unsupported -k-|-no-main with executable output type");
        exit(ExitCode::ArgParseError);
    }

    if options.host_os == HostOS::MacOS && options.body_type == BodyType::MainGen {
        eprintln!("Linking the C standard library from a kernel+main module is not supported on MacOS");
        eprintln!("Please use the C-derived main option (-C|--c-main)");
        exit(ExitCode::ArgParseError);
    }
}

fn parse_args<'a>(
    args: &'a [String],
    input: &mut InputType<'a>,
    output: &mut OutputType<'a>,
    options: &mut RunOptions
) {
    let _bin_name: &String = args.first().unwrap();
    let mut arg: &'a String;
    let mut i: usize = 1;

    while i < args.len() {
        arg = args.get(i).unwrap();
        match arg.as_str() {
            "--ast"         => options.print_ast = true,
            "-b"            => set_codegen_type(options, CodeGenType::Bitcode),
            "--bitcode"     => set_codegen_type(options, CodeGenType::Bitcode),
            "-c"            => set_codegen_type(options, CodeGenType::Object),
            "-C"            => set_body_type(options, BodyType::MainGenC),
            "--drop"        => options.drop_token = true,
            "-e"            => *input = InputType::Expr(parse_arg_after(args, &mut i)),
            "--expr"        => *input = InputType::Expr(parse_arg_after(args, &mut i)),
            "-h"            => help(ExitCode::Ok),
            "--help"        => help(ExitCode::Ok),
            "--ir"          => options.ir_exit = true,
            "-k"            => set_body_type(options, BodyType::NoMain),
            "--lex"         => options.lex_exit = true,
            "--llvmir"      => set_codegen_type(options, CodeGenType::Llvmir),
            "--no-main"     => set_body_type(options, BodyType::NoMain),
            "--notarget"    => options.no_target = true,
            "-o"            => *output = OutputType::File(parse_arg_after(args, &mut i)),
            "-O0"           => options.opt_level = OptLevel::O0,
            "-O1"           => options.opt_level = OptLevel::O1,
            "-O2"           => options.opt_level = OptLevel::O2,
            "-O3"           => options.opt_level = OptLevel::O3,
            "--parse"       => options.parse_exit = true,
            "--sem"         => options.sem_exit = true,
            "-S"            => set_codegen_type(options, CodeGenType::Llvmir),
            "--c-main"      => set_body_type(options, BodyType::MainGenC),
            "-v"            => options.verbose = true,
            "--verbose"     => options.verbose = true,
            "--version"     => print_pkg_info(true),
            _               => parse_arg_complex(arg, input, output),
        }
        i += 1;
    }

    if options.body_type == BodyType::Unset {
        set_body_type(options, BodyType::MainGen);
    }

    if options.codegen_type == CodeGenType::Unset {
        set_codegen_type(options, CodeGenType::Executable);
    }

    if *input == InputType::None {
        eprintln!("No input file/name specified!");
        help(ExitCode::ArgParseError);
    } else if options.verbose {
        eprintln!("Processing input '{}'", *input);
    }

    if options.verbose {
        eprintln!("Outputting to '{}'", *output);
    }

    check_options_configuration(options, output);
    if options.verbose {
        eprintln!("{}", options);
    }
}

fn parse_arg_after<'a>(args: &'a [String], i: &mut usize) -> &'a str {
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
    } else if *input != InputType::None {
        eprintln!("Found more than one input ('{}' and '{}')", *input, arg);
        help(ExitCode::ArgParseError);
    } else if arg.len() == 1 && lead_char == '-' {
        *input = InputType::Stdin;
    } else {
        *input = InputType::File(arg.as_str());
    }
}

fn print_pkg_info(should_exit: bool) {
    eprintln!("Welcome to {} version {}\n{}", PACKAGE, VERSION, LICENSE);
    if should_exit { exit(ExitCode::Ok); }
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
    let irgen_status: bool = IRGen::gen(*ast, &mut module_irgen);
    assert!(irgen_status);
    let irgen_verify: bool = module_irgen.verify_module();
    if !irgen_verify {
        eprintln!("IRGen module failed to verify");
        exit(ExitCode::VerifyError);
    }

    let module_name_main = String::from("main");
    let mut module_main = match options.body_type {
        BodyType::NoMain    => module_irgen,
        BodyType::MainGen   => {
            let mut module_main = ModuleBundle::new(&module_name_main, options.verbose);

            // NOTE: Fixing the verifier errors by reconstructing the callee signature using
            // the correct main module context causes the link step to omit the body of the callee
            // (emitting only a declaration).
            //let f_sig = module_main.get_f_sig_from_context(&module_irgen.f_sig.clone().unwrap());
            let f_sig = &module_irgen.f_sig.clone().unwrap();

            let maingen_status: bool = MainGen::gen(&mut module_main, f_sig);
            if !maingen_status {
                eprintln!("Failed to generate MainGen module");
                exit(ExitCode::MainGenError);
            }

            // NOTE: Linking modules causes verifier errors for unmatched contexts of LLVMTypeRef.
            // Skip verification.
            let link_status: bool = module_main.link_into(&mut module_irgen);
            if !link_status {
                eprintln!("IRGen and MainGen modules failed to link");
                exit(ExitCode::LinkError);
            }

            module_main
        },
        BodyType::MainGenC  => {
            // Generating the main.c object need to happen until after the target/optimization step,
            // but to appease the borrow checker, we do it here.
            let f_sig = &module_irgen.f_sig.clone().unwrap();
            let stem = format!("{}pid{}",
                match input {
                    InputType::File(f)  => Path::new(f).file_stem().unwrap().to_str().unwrap(),
                    _                   => "",
                },
                process::id()
            );
            let main_obj_path = match MainGenC::gen(f_sig, &stem, options.verbose) {
                Some(path)  => path,
                None        => {
                    eprintln!("Failed to generate MainGenC object file");
                    exit(ExitCode::MainGenCError);
                },
            };

            module_irgen.push_object(main_obj_path);
            module_irgen
        },
        BodyType::Unset     => {
            eprintln!("Invalid body type");
            exit(ExitCode::ModuleError);
        },
    };

    match input {
        InputType::File(f)  => module_main.set_sourcefile_name(f),
        _                   => module_main.set_sourcefile_name("-"),
    };

    if options.ir_exit {
        eprintln!("{}", module_main);
        exit(ExitCode::Ok);
    }

    let mut target = Target::new();
    if options.verbose {
        eprintln!("Detected target triple '{}'", target.get_string());
    }
    let mut machine = TargetMachine::new(&mut target, options.opt_level);
    let mut pass_builder = PassBuilder::new();
    let opt_result = pass_builder.run(
        &mut module_main,
        &mut machine,
        Passes::Default(options.opt_level),
        options.no_target
    );
    if !opt_result {
        exit(ExitCode::TargetError);
    }

    let write_result = module_main.write_module(&options, &output);
    if !write_result {
        eprintln!("Failed to write module to output");
        exit(ExitCode::WriteError);
    }

    exit(ExitCode::Ok);
}
