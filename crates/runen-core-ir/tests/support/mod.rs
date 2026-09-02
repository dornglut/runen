use runen_core_ir::{Body, Function, Program, SafeReferenceResultContract, TypeTable};

pub fn one_function_program(types: TypeTable, body: Body) -> Program {
    Program {
        types,
        functions: vec![Function {
            name: "one_function_regression".into(),
            parameters: Vec::new(),
            result: None,
            safe_reference_result_contract: SafeReferenceResultContract::None,
            body,
        }],
    }
}
