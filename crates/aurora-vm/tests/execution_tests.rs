//! End-to-end integration tests for Aurora compiler and Virtual Machine.

use aurora_lexer::Lexer;
use aurora_parser::Parser;
use aurora_sema::SemanticAnalyzer;
use aurora_vm::{BytecodeCompiler, Value, Vm};

fn run_aurora_source(source: &str) -> (Result<Value, String>, Option<Vec<String>>) {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize();

    let mut parser = Parser::new(tokens);
    let program = match parser.parse_program() {
        Ok(p) => p,
        Err(e) => return (Err(format!("Parse error: {e}")), None),
    };

    let mut sema = SemanticAnalyzer::new();
    if let Err(errors) = sema.analyze_program(&program) {
        let err_msgs = errors
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n");
        return (Err(format!("Semantic errors: {err_msgs}")), None);
    }

    let mut compiler = BytecodeCompiler::new();
    let compiled = match compiler.compile(&program) {
        Ok(c) => c,
        Err(e) => return (Err(format!("Compile error: {e}")), None),
    };

    let mut vm = Vm::new().with_output_capture();
    let result = vm.run_main(compiled).map_err(|e| format!("VM error: {e}"));
    let captured = vm.captured_lines();

    (result, captured)
}

#[test]
fn test_end_to_end_factorial_recursion() {
    let src = r"
    fn factorial(n: i64) -> i64 {
        if n <= 1 {
            return 1;
        } else {
            return n * factorial(n - 1);
        }
    }

    fn main() -> i64 {
        return factorial(5);
    }
    ";

    let (res, _) = run_aurora_source(src);
    assert_eq!(res.unwrap(), Value::Int(120));
}

#[test]
fn test_end_to_end_while_loop_sum() {
    let src = r"
    fn sum_to(limit: i64) -> i64 {
        let mut total: i64 = 0;
        let mut i: i64 = 1;
        while i <= limit {
            total += i;
            i += 1;
        }
        return total;
    }

    fn main() -> i64 {
        return sum_to(100);
    }
    ";

    let (res, _) = run_aurora_source(src);
    assert_eq!(res.unwrap(), Value::Int(5050));
}

#[test]
fn test_end_to_end_pipeline_operator() {
    let src = r"
    fn double(x: i64) -> i64 {
        return x * 2;
    }

    fn add_three(x: i64) -> i64 {
        return x + 3;
    }

    fn main() -> i64 {
        let result: i64 = 5 |> double() |> add_three();
        return result;
    }
    ";

    let (res, _) = run_aurora_source(src);
    assert_eq!(res.unwrap(), Value::Int(13));
}

#[test]
fn test_end_to_end_unsafe_pointer_operations() {
    let src = r"
    unsafe fn test_pointers() -> i64 {
        let ptr: *mut i64 = alloc(8);
        *ptr = 1337;
        let read_val: i64 = *ptr;
        free(ptr);
        return read_val;
    }

    fn main() -> i64 {
        return unsafe { test_pointers() };
    }
    ";

    let (res, _) = run_aurora_source(src);
    assert_eq!(res.unwrap(), Value::Int(1337));
}

#[test]
fn test_end_to_end_array_indexing() {
    let src = r"
    fn main() -> i64 {
        let numbers: [i64] = [10, 20, 30, 40];
        return numbers[2];
    }
    ";

    let (res, _) = run_aurora_source(src);
    assert_eq!(res.unwrap(), Value::Int(30));
}

#[test]
fn test_end_to_end_captured_stdout() {
    let src = r#"
    fn main() {
        println("Aurora Language Engine:", "v0.1.0");
        print("Status:", "Operational");
    }
    "#;

    let (res, output) = run_aurora_source(src);
    assert!(res.is_ok());
    let out = output.unwrap();
    assert_eq!(out.len(), 2);
    assert_eq!(out[0], "Aurora Language Engine: v0.1.0");
    assert_eq!(out[1], "Status: Operational");
}

#[test]
fn test_end_to_end_for_range_loop() {
    let src = r"
    fn main() -> i64 {
        let mut sum: i64 = 0;
        for i in 1..=10 {
            sum += i;
        }
        return sum;
    }
    ";

    let (res, _) = run_aurora_source(src);
    assert_eq!(res.unwrap(), Value::Int(55));
}

#[test]
fn test_end_to_end_match_statement() {
    let src = r"
    fn classify(x: i64) -> i64 {
        return match x {
            1 => 100,
            2 => 200,
            _ => 999,
        };
    }

    fn main() -> i64 {
        return classify(2);
    }
    ";

    let (res, _) = run_aurora_source(src);
    assert_eq!(res.unwrap(), Value::Int(200));
}
