#   Copyright

Copyright 2024, Giordano Salvador
SPDX-License-Identifier: BSD-3-Clause

Author/Maintainer:  Giordano Salvador <73959795+e3m3@users.noreply.github.com>


#   Description (calcc language)

[![Ubuntu 22.04](https://github.com/e3m3/calcc-rust/actions/workflows/ubuntu-2204.yaml/badge.svg?event=workflow_dispatch)](https://github.com/e3m3/calcc-rust/actions/workflows/ubuntu-2204.yaml)
[![Ubuntu 24.04](https://github.com/e3m3/calcc-rust/actions/workflows/ubuntu-2404.yaml/badge.svg?event=workflow_dispatch)](https://github.com/e3m3/calcc-rust/actions/workflows/ubuntu-2404.yaml)
[![Fedora 40](https://github.com/e3m3/calcc-rust/actions/workflows/fedora-40.yaml/badge.svg?event=workflow_dispatch)](https://github.com/e3m3/calcc-rust/actions/workflows/fedora-40.yaml)

[![MacOS 13](https://github.com/e3m3/calcc-rust/actions/workflows/macos-13.yaml/badge.svg?event=workflow_dispatch)](https://github.com/e3m3/calcc-rust/actions/workflows/macos-13.yaml)
[![MacOS 14](https://github.com/e3m3/calcc-rust/actions/workflows/macos-14.yaml/badge.svg?event=workflow_dispatch)](https://github.com/e3m3/calcc-rust/actions/workflows/macos-14.yaml)

[![Windows 2022](https://github.com/e3m3/calcc-rust/actions/workflows/windows-2022.yaml/badge.svg?event=workflow_dispatch)](https://github.com/e3m3/calcc-rust/actions/workflows/windows-2022.yaml)

Learning [Rust][1] [[1]] by implementing the calc langauge using the [llvm-sys][2] [[2]] crate.
Implements the calc language, inspired by the [C++][3] [[3]] implementation presented by
    Macke and Kwan in [[4]] and [[5]].

Accepted factors in the grammar have been extended for convenience (see `src/{lex,parse}.rs`
    and `tests/lit-tests/`).
The output of the compiler is LLVM IR, LLVM bitcode, an object file, or executable file [[6]].


##  Language

### Lexer

```text
ident           ::= is_letter+ (is_letter | is_number)*
number          ::= digit+ | (`0x` hex_digit+)
digit           ::= [0-9]
hex_digit       ::= [a-fA-F0-9]
letter          ::= letter_lower | letter_upper | `_`
letter_lower    ::= [a-z]
letter_upper    ::= [A-Z]
whitespace      ::= ` ` | `\r` | `\n` | `\t`

any             ::= _
token           ::= { tokenkind, text }
tokenkind       ::=
    | Unknown
    | Comma
    | Comment
    | Colon
    | Eoi
    | Eol
    | Ident
    | Minus
    | Number
    | ParenL
    | ParenR
    | Plus
    | Slash
    | Star
    | With
text            ::=
    | ``
    | `,`
    | `/``/` any*
    | `:`
    | ident
    | `-`
    | number
    | `(`
    | `)`
    | `+`
    | `/`
    | `*`
    | `with`
```

### Grammar

```text
calc    ::= ( With Colon Ident (Comma Ident)* Colon )? expr
expr    ::= term ( Plus | Minus ) term
factor  ::= Minus? ( Number | Ident | ParenL expr ParenR )
term    ::= factor ( Slash | Star ) factor
```

Notes:

*   The grammar rules above use the `tokenkind` as a shorthand for a `token` object as described
    by the lexer rules.

*   In the AST, a factor with a leading `Minus` token is represented as a subtraction expression
    where the left term is `Number` with the constant value `0`.


##  Prerequisites

*   libstdc++

*   rust-2021

*   llvm-18 and llvm-sys (or llvm version matching llvm-sys)

*   clang-18 (for executables and '-C|--c-main' flags)

*   python3-lit, FileCheck (for testing)

    *   By default, `tests/lit-tests.rs` will search for the lit executable in
        `$PYTHON_VENV_PATH/bin` (if it exists) or the system's `/usr/bin`.

*   [docker|podman] (for testing/containerization)

    *   A [Fedora][7] [[7]] image can be built using `containers/Containerfile.fedora*`.

    *   An [Ubuntu][8] [[8]] image can be built using `containers/Containerfile.ubuntu*`.


##  Setup

*   Native build and test:
    
    ```shell
    cargo build
    cargo test -- --nocapture
    ```

*   Container build and test [podman][9] [[9]]:

    ```shell
    podman build -t calcc -f container/Containerfile .
    ```

*   Container build and test [docker][10] [[10]]:

    ```shell
    docker build -t calcc -f container/Dockerfile .
    ```

*   If `make` is installed, you can build the image by running:

    ```shell
    make
    ```

##   Usage

From the help message (`calcc --help`):

```
usage: calcc [OPTIONS] <INPUT>
INPUT              '-' (i.e., Stdin) or a file path
OPTIONS:
--ast              Print the AST after parsing
-b|--bitcode       Output LLVM bitcode (post-optimization) (.bc if used with -o)
-c                 Output an object file (post-optimization) (.o if used with -o)
--drop             Drop unknown tokens instead of failing
-e|--expr[=]<E>    Process expression E instead of INPUT file
-h|--help          Print this list of command line options
--lex              Exit after running the lexer
--ir               Exit after printing IR (pre-optimization)
-S|--llvmir        Output LLVM IR (post-optimization) (.ll if used with -o)
-k|--no-main       Omit linking with main module (i.e., output kernel only)
                   When this option is selected, an executable cannot be generated
--notarget         Omit target specific configuration in LLVM IR/bitcode
-o[=]<F>           Output to file F instead of Stdout
                   If no known extension is used (.bc|.exe|.ll|.o) an executable is assumed
                   An executable requires llc and clang to be installed
-O<0|1|2|3>        Set the optimization level (default: O2)
--parse            Exit after running the parser
--sem              Exit after running the semantics check
-C|--c-main        Link with a C-derived main module (src/main.c.template)
                   This option is required for generating object files and executables on MacOS
                   and requires clang to be installed
-v|--verbose       Enable verbose output
--version          Display the package version and license information
```


#   References

[1]:    https://www.rust-lang.org/
[2]:    https://crates.io/crates/llvm-sys
[3]:    https://isocpp.org/
[4]:    https://www.packtpub.com/product/learn-llvm-17-second-edition/9781837631346
[5]:    https://github.com/PacktPublishing/Learn-LLVM-17
[6]:    https://llvm.org/
[7]:    https://fedoraproject.org/
[8]:    https://ubuntu.com/
[9]:    https://podman.io/
[10]:   https://www.docker.com/

1.  `https://www.rust-lang.org/`

1.  `https://crates.io/crates/llvm-sys`

1.  `https://isocpp.org/`

1.  `https://www.packtpub.com/product/learn-llvm-17-second-edition/9781837631346`

1.  `https://github.com/PacktPublishing/Learn-LLVM-17`

1.  `https://llvm.org/`

1.  `https://fedoraproject.org/`

1.  `https://ubuntu.com/`

1.  `https://podman.io/`

1.  `https://www.docker.com/`
