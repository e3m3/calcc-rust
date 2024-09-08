#   Copyright

Copyright 2024, Giordano Salvador
SPDX-License-Identifier: BSD-3-Clause

Author/Maintainer:  Giordano Salvador <73959795+e3m3@users.noreply.github.com>


#   Description (calcc language)

Learning [Rust][1] [[1]] by implementing the calc langauge using the [llvm-sys][2] [[2]] crate.
Implements the calc language, inspired by the [C++][3] [[3]] implementation presented by Macke and Kwan in [[4]] and [[5]].

Accepted factors in the grammar have been extended for convenience (see `src/{lex,parse}.rs` and `tests/lit-llvm/`).
The output of the compiler is LLVM IR or LLVM bytecode [[6]].


##  Grammar

```text
calc : ("with" ":" ident ("," ident)* ":" )? expr
```


##  Prequisites

*   libstdc++

*   rust-2021

*   llvm18 and llvm-sys (or llvm version matching llvm-sys)

*   python3-lit, FileCheck, clang (for testing)

    *   By default, `tests/lit-tests-llvm.rs` will search for the lit executable in the `$PYTHON_VENV_PATH/bin`
        (if it exists) or the system's `/usr/bin`.

*   [docker|podman] (for testing/containerization)

    *   A [Fedora][7] [[7]] image can be built using `containers/Containerfile.fedora`.

    *   An [Ubuntu][8] [[8]] image can be built using `containers/Containerfile.ubuntu`.


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
