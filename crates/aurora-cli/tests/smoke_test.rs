use aurora_lexer::Lexer;
use aurora_parser::Parser;
use aurora_vm::{Chunk, OpCode, Value, Vm};

#[test]
fn test_toolchain_smoke() {
    let source = "let answer = 40 + 2";
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize();

    let mut parser = Parser::new(tokens);
    let program = parser.parse_program().expect("Parsing failed");
    assert_eq!(program.statements.len(), 1);

    // Verify VM execution
    let mut vm = Vm::new();
    let mut chunk = Chunk::new();
    let c1 = chunk.add_constant(Value::Int(40));
    let c2 = chunk.add_constant(Value::Int(2));
    chunk.write_opcode(OpCode::Constant(c1), 1);
    chunk.write_opcode(OpCode::Constant(c2), 1);
    chunk.write_opcode(OpCode::Add, 1);
    chunk.write_opcode(OpCode::Return, 1);

    let res = vm.run(&chunk).expect("VM run failed");
    assert_eq!(res, Value::Int(42));
}
