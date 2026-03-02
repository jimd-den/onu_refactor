# Usage

## 1. Write a program
Create a file (e.g., `hello.onu`) with the following structure:

```onu
the module called MyModule
    with concern: my first program

the effect behavior called run
    takes: nothing
    delivers: nothing
    as:
        broadcasts "Hello, World!"
        nothing
```

## 2. Compile and Run
Run these commands to generate and execute the binary:

```bash
cargo run -- hello.onu -o hello.ll
clang hello.ll -O3 -o hello_bin -Wno-override-module
./hello_bin
```
