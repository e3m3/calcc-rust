#   Copyright

Copyright 2024, Giordano Salvador
SPDX-License-Identifier: BSD-3-Clause

Author/Maintainer:  Giordano Salvador <73959795+e3m3@users.noreply.github.com>


#   Description (calcc language)

Learning rust by implementing the calc langauge using the llvm-sys crate.
Implements the calc language, inspired by the C++ implementation presented by Macke and Kwan in [1][1] and [2][2].


##  Grammar

```text
calc : ("with" ":" ident ("," ident)* ":" )? expr ";"
```


##  Prequisites

*   rust-2021

*   llvm-17

*   python3-lit, FileCheck

*   [docker|podman]


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
