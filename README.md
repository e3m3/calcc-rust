#   Copyright

Copyright 2024, Giordano Salvador
SPDX-License-Identifier: BSD-3-Clause

Author/Maintainer:  Giordano Salvador <73959795+e3m3@users.noreply.github.com>


#   Description (calcc language)

Learning rust by implementing the calc langauge using the llvm-sys crate.
Implements the calc language, inspired by the C++ implementation presented by Macke and Kwan in [1][1] and [2][2].

Accepted factors in the grammar have been extended for convenience (see 'src/{lex,parse}.rs' and 'tests/lit-llvm/').
The output of the compiler is LLVM IR or LLVM bytecode.


##  Grammar

```text
calc : ("with" ":" ident ("," ident)* ":" )? expr
```


##  Prequisites

*   libstdc++

*   rust-2021

*   llvm18 and llvm-sys (or llvm version matching llvm-sys)

*   python3-lit, FileCheck, clang (for testing)

*   [docker|podman] (for testing/containerization)


##  Setup

*   Native build and test:
    
    ```shell
    cargo build
    cargo test -- --nocapture
    ```

*   Container build and test (podman):

    ```shell
    podman build -t calcc -f container/Containerfile .
    ```

*   Container build and test (docker):

    ```shell
    docker build -t calcc -f container/Dockerfile .
    ```

*   If `make` is installed, you can build the image by running:

    ```shell
    make
    ```


#   References

[1]:    https://www.packtpub.com/product/learn-llvm-17-second-edition/9781837631346
[2]:    https://github.com/PacktPublishing/Learn-LLVM-17

1.  `https://www.packtpub.com/product/learn-llvm-17-second-edition/9781837631346`

1.  `https://github.com/PacktPublishing/Learn-LLVM-17`
