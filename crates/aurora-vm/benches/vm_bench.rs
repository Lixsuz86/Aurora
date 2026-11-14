//! High-resolution micro-benchmark suite for Aurora Bytecode Virtual Machine.

use aurora_lexer::Lexer;
use aurora_parser::Parser;
use aurora_sema::SemanticAnalyzer;
use aurora_vm::{BytecodeCompiler, CompiledFunction, Vm};
use std::rc::Rc;
use std::time::Instant;

fn compile_source(source: &str) -> Rc<CompiledFunction> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program().expect("Parse failed");
    let mut sema = SemanticAnalyzer::new();
    sema.analyze_program(&program).expect("Sema failed");
    let mut compiler = BytecodeCompiler::new();
    compiler.compile(&program).expect("Compilation failed")
}

fn bench_arithmetic_loop() {
    let src = r"
    fn main() -> i64 {
        let mut sum: i64 = 0;
        let mut i: i64 = 0;
        while i < 1000000 {
            sum += i;
            i += 1;
        }
        return sum;
    }
    ";

    let compiled = compile_source(src);
    let mut vm = Vm::new();

    // Warm-up run
    let _ = vm.run_main(compiled.clone());

    let start = Instant::now();
    let result = vm.run_main(compiled).expect("Run failed");
    let elapsed = start.elapsed();

    // 1M iterations * ~6 opcodes per iteration = ~6M opcodes
    let opcodes = 6_000_000_f64;
    let mops = (opcodes / elapsed.as_secs_f64()) / 1_000_000.0;

    println!(
        "Benchmark [Arithmetic Tight Loop 1M]: {elapsed:?} (Result: {result}) -> {mops:.2} MOps/sec"
    );
}

fn bench_recursive_fibonacci() {
    let src = r"
    fn fib(n: i64) -> i64 {
        if n <= 1 {
            return n;
        }
        return fib(n - 1) + fib(n - 2);
    }

    fn main() -> i64 {
        return fib(20);
    }
    ";

    let compiled = compile_source(src);
    let mut vm = Vm::new();

    let start = Instant::now();
    let result = vm.run_main(compiled).expect("Run failed");
    let elapsed = start.elapsed();

    // fib(20) makes 21,891 function calls
    let calls = 21_891_f64;
    let calls_per_sec = calls / elapsed.as_secs_f64();

    println!(
        "Benchmark [Recursive Fibonacci fib(20)]: {elapsed:?} (Result: {result}) -> {calls_per_sec:.0} calls/sec"
    );
}

fn bench_pointer_memory_ops() {
    let src = r"
    unsafe fn run_pointer_loop() -> i64 {
        let ptr: *mut i64 = alloc(8);
        let mut i: i64 = 0;
        let mut last: i64 = 0;
        while i < 100000 {
            *ptr = i;
            last = *ptr;
            i += 1;
        }
        free(ptr);
        return last;
    }

    fn main() -> i64 {
        return unsafe { run_pointer_loop() };
    }
    ";

    let compiled = compile_source(src);
    let mut vm = Vm::new();

    let start = Instant::now();
    let result = vm.run_main(compiled).expect("Run failed");
    let elapsed = start.elapsed();

    let accesses = 200_000_f64; // 100k stores + 100k loads
    let maccess_per_sec = (accesses / elapsed.as_secs_f64()) / 1_000_000.0;

    println!(
        "Benchmark [Raw Pointer Store/Load 100k]: {elapsed:?} (Result: {result}) -> {maccess_per_sec:.2} M-Access/sec"
    );
}

fn main() {
    println!("\n=======================================================");
    println!("       Aurora Bytecode VM Micro-Benchmark Suite       ");
    println!("=======================================================\n");

    bench_arithmetic_loop();
    bench_recursive_fibonacci();
    bench_pointer_memory_ops();

    println!("\n=======================================================");
    println!("             Micro-Benchmark Complete!                 ");
    println!("=======================================================\n");
}
