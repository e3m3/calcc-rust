#   Copyright

Copyright 2024, Giordano Salvador
SPDX-License-Identifier: BSD-3-Clause

Author/Maintainer:  Giordano Salvador <73959795+e3m3@users.noreply.github.com>


#   Description (calcc language)

[![Ubuntu 22.04](https://github.com/e3m3/calcc-rust/actions/workflows/ubuntu-2204.yaml/badge.svg?event=workflow_dispatch)](https://github.com/e3m3/calcc-rust/actions/workflows/ubuntu-2204.yaml)
[![Ubuntu 24.04](https://github.com/e3m3/calcc-rust/actions/workflows/ubuntu-2404.yaml/badge.svg?event=workflow_dispatch)](https://github.com/e3m3/calcc-rust/actions/workflows/ubuntu-2404.yaml)
[![Fedora 40](https://github.com/e3m3/calcc-rust/actions/workflows/fedora-40.yaml/badge.svg?event=workflow_dispatch)](https://github.com/e3m3/calcc-rust/actions/workflows/fedora-40.yaml)

[![MacOS 13 (Container)](https://github.com/e3m3/calcc-rust/actions/workflows/macos-13-container.yaml/badge.svg?event=workflow_dispatch)](https://github.com/e3m3/calcc-rust/actions/workflows/macos-13-container.yaml)
[![MacOS 14 (Native)](https://github.com/e3m3/calcc-rust/actions/workflows/macos-14-native.yaml/badge.svg?event=workflow_dispatch)](https://github.com/e3m3/calcc-rust/actions/workflows/macos-14-native.yaml)

Learning [Rust][1] [[1]] by implementing the calc langauge using the [llvm-sys][2] [[2]] crate.
Implements the calc language, inspired by the [C++][3] [[3]] implementation presented by Macke and Kwan in [[4]] and [[5]].

Accepted factors in the grammar have been extended for convenience (see `src/{lex,parse}.rs` and `tests/lit-llvm/`).
The output of the compiler is LLVM IR or LLVM bytecode [[6]].


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

*   The grammar rules above use the `tokenkind` as a shorthand for a `token` object as described by the lexer rules.

*   In the AST, a factor with a leading `Minus` token is represented as a subtraction expression where the left term
    is `Number` with the constant value `0`.


##  Prequisites

*   libstdc++

*   rust-2021

*   llvm18 and llvm-sys (or llvm version matching llvm-sys)

*   python3-lit, FileCheck, clang (for testing)

    *   By default, `tests/lit-tests-llvm.rs` will search for the lit executable in the `$PYTHON_VENV_PATH/bin`
        (if it exists) or the system's `/usr/bin`.

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
--drop             Drop unknown tokens instead of failing
-e|--expr[=]<E>    Process expression E instead of INPUT file
-h|--help          Print this list of command line options
--lex              Exit after running the lexer
-S|--llvmir        Exit after outputting LLVM IR (post-optimization) instead of byte code
--nomain           Omit linking with main module (i.e., output kernel only)
--notarget         Omit target specific configuration in LLVM IR/bytecode
-o[=]<F>           Output to LLVM IR (.ll) or bytecode (.bc) file F instead of Stdout
-O<0|1|2|3>        Set the optimization level (default: O2)
--parse            Exit after running the parser
--sem              Exit after running the semantics check
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
