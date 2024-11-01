use naga::{valid, Expression, Function, Scalar};

/// Validation should fail if `AtomicResult` expressions are not
/// populated by `Atomic` statements.
#[test]
fn populate_atomic_result() {
    use naga::{Module, Type, TypeInner};

    /// Different variants of the test case that we want to exercise.
    enum Variant {
        /// An `AtomicResult` expression with an `Atomic` statement
        /// that populates it: valid.
        Atomic,

        /// An `AtomicResult` expression visited by an `Emit`
        /// statement: invalid.
        Emit,

        /// An `AtomicResult` expression visited by no statement at
        /// all: invalid
        None,
    }

    // Looking at uses of `variant` should make it easy to identify
    // the differences between the test cases.
    fn try_variant(
        variant: Variant,
    ) -> Result<naga::valid::ModuleInfo, naga::WithSpan<naga::valid::ValidationError>> {
        let span = naga::Span::default();
        let mut module = Module::default();
        let ty_u32 = module.types.insert(
            Type {
                name: Some("u32".into()),
                inner: TypeInner::Scalar(Scalar::U32),
            },
            span,
        );
        let ty_atomic_u32 = module.types.insert(
            Type {
                name: Some("atomic<u32>".into()),
                inner: TypeInner::Atomic(Scalar::U32),
            },
            span,
        );
        let var_atomic = module.global_variables.append(
            naga::GlobalVariable {
                name: Some("atomic_global".into()),
                space: naga::AddressSpace::WorkGroup,
                binding: None,
                ty: ty_atomic_u32,
                init: None,
            },
            span,
        );

        let mut fun = Function::default();
        let ex_global = fun
            .expressions
            .append(Expression::GlobalVariable(var_atomic), span);
        let ex_42 = fun
            .expressions
            .append(Expression::Literal(naga::Literal::U32(42)), span);
        let ex_result = fun.expressions.append(
            Expression::AtomicResult {
                ty: ty_u32,
                comparison: false,
            },
            span,
        );

        match variant {
            Variant::Atomic => {
                fun.body.push(
                    naga::Statement::Atomic {
                        pointer: ex_global,
                        fun: naga::AtomicFunction::Add,
                        value: ex_42,
                        result: Some(ex_result),
                    },
                    span,
                );
            }
            Variant::Emit => {
                fun.body.push(
                    naga::Statement::Emit(naga::Range::new_from_bounds(ex_result, ex_result)),
                    span,
                );
            }
            Variant::None => {}
        }

        module.functions.append(fun, span);

        valid::Validator::new(
            valid::ValidationFlags::default(),
            valid::Capabilities::all(),
        )
        .validate(&module)
    }

    try_variant(Variant::Atomic).expect("module should validate");
    assert!(try_variant(Variant::Emit).is_err());
    assert!(try_variant(Variant::None).is_err());
}

#[test]
fn populate_call_result() {
    use naga::{Module, Type, TypeInner};

    /// Different variants of the test case that we want to exercise.
    enum Variant {
        /// A `CallResult` expression with an `Call` statement that
        /// populates it: valid.
        Call,

        /// A `CallResult` expression visited by an `Emit` statement:
        /// invalid.
        Emit,

        /// A `CallResult` expression visited by no statement at all:
        /// invalid
        None,
    }

    // Looking at uses of `variant` should make it easy to identify
    // the differences between the test cases.
    fn try_variant(
        variant: Variant,
    ) -> Result<naga::valid::ModuleInfo, naga::WithSpan<naga::valid::ValidationError>> {
        let span = naga::Span::default();
        let mut module = Module::default();
        let ty_u32 = module.types.insert(
            Type {
                name: Some("u32".into()),
                inner: TypeInner::Scalar(Scalar::U32),
            },
            span,
        );

        let mut fun_callee = Function {
            result: Some(naga::FunctionResult {
                ty: ty_u32,
                binding: None,
            }),
            ..Function::default()
        };
        let ex_42 = fun_callee
            .expressions
            .append(Expression::Literal(naga::Literal::U32(42)), span);
        fun_callee
            .body
            .push(naga::Statement::Return { value: Some(ex_42) }, span);
        let fun_callee = module.functions.append(fun_callee, span);

        let mut fun_caller = Function::default();
        let ex_result = fun_caller
            .expressions
            .append(Expression::CallResult(fun_callee), span);

        match variant {
            Variant::Call => {
                fun_caller.body.push(
                    naga::Statement::Call {
                        function: fun_callee,
                        arguments: vec![],
                        result: Some(ex_result),
                    },
                    span,
                );
            }
            Variant::Emit => {
                fun_caller.body.push(
                    naga::Statement::Emit(naga::Range::new_from_bounds(ex_result, ex_result)),
                    span,
                );
            }
            Variant::None => {}
        }

        module.functions.append(fun_caller, span);

        valid::Validator::new(
            valid::ValidationFlags::default(),
            valid::Capabilities::all(),
        )
        .validate(&module)
    }

    try_variant(Variant::Call).expect("should validate");
    assert!(try_variant(Variant::Emit).is_err());
    assert!(try_variant(Variant::None).is_err());
}

#[test]
fn emit_workgroup_uniform_load_result() {
    use naga::{Module, Type, TypeInner};

    // We want to ensure that the *only* problem with the code is the
    // use of an `Emit` statement instead of an `Atomic` statement. So
    // validate two versions of the module varying only in that
    // aspect.
    //
    // Looking at uses of the `wg_load` makes it easy to identify the
    // differences between the two variants.
    fn variant(
        wg_load: bool,
    ) -> Result<naga::valid::ModuleInfo, naga::WithSpan<naga::valid::ValidationError>> {
        let span = naga::Span::default();
        let mut module = Module::default();
        let ty_u32 = module.types.insert(
            Type {
                name: Some("u32".into()),
                inner: TypeInner::Scalar(Scalar::U32),
            },
            span,
        );
        let var_workgroup = module.global_variables.append(
            naga::GlobalVariable {
                name: Some("workgroup_global".into()),
                space: naga::AddressSpace::WorkGroup,
                binding: None,
                ty: ty_u32,
                init: None,
            },
            span,
        );

        let mut fun = Function::default();
        let ex_global = fun
            .expressions
            .append(Expression::GlobalVariable(var_workgroup), span);
        let ex_result = fun
            .expressions
            .append(Expression::WorkGroupUniformLoadResult { ty: ty_u32 }, span);

        if wg_load {
            fun.body.push(
                naga::Statement::WorkGroupUniformLoad {
                    pointer: ex_global,
                    result: ex_result,
                },
                span,
            );
        } else {
            fun.body.push(
                naga::Statement::Emit(naga::Range::new_from_bounds(ex_result, ex_result)),
                span,
            );
        }

        module.functions.append(fun, span);

        valid::Validator::new(
            valid::ValidationFlags::default(),
            valid::Capabilities::all(),
        )
        .validate(&module)
    }

    variant(true).expect("module should validate");
    assert!(variant(false).is_err());
}

#[cfg(feature = "wgsl-in")]
#[test]
fn bad_cross_builtin_args() {
    let cases = [
        (
            "vec2(0., 1.)",
            "\
error: Entry point main at Compute is invalid
  ┌─ wgsl:3:13
  │
3 │     let a = cross(vec2(0., 1.), vec2(0., 1.));
  │             ^^^^^ naga::Expression [6]
  │
  = Expression [6] is invalid
  = Argument [0] to Cross as expression [2] has an invalid type.

",
        ),
        (
            "vec4(0., 1., 2., 3.)",
            "\
error: Entry point main at Compute is invalid
  ┌─ wgsl:3:13
  │
3 │     let a = cross(vec4(0., 1., 2., 3.), vec4(0., 1., 2., 3.));
  │             ^^^^^ naga::Expression [10]
  │
  = Expression [10] is invalid
  = Argument [0] to Cross as expression [4] has an invalid type.

",
        ),
    ];

    for (invalid_arg, expected_err) in cases {
        let source = format!(
            "\
@compute @workgroup_size(1)
fn main() {{
    let a = cross({invalid_arg}, {invalid_arg});
}}
"
        );
        let module = naga::front::wgsl::parse_str(&source).unwrap();
        let err = valid::Validator::new(Default::default(), valid::Capabilities::all())
            .validate_no_overrides(&module)
            .expect_err("module should be invalid");
        assert_eq!(err.emit_to_string(&source), expected_err);
    }
}

#[cfg(feature = "wgsl-in")]
#[test]
fn validation_error_messages() {
    let cases = [(
        r#"@group(0) @binding(0) var my_sampler: sampler;

                fn foo(tex: texture_2d<f32>) -> vec4<f32> {
                    return textureSampleLevel(tex, my_sampler, vec2f(0, 0), 0.0);
                }

                fn main() {
                    foo();
                }
            "#,
        "\
error: Function [1] 'main' is invalid
  ┌─ wgsl:7:17
  │  \n7 │ ╭                 fn main() {
8 │ │                     foo();
  │ │                     ^^^^ invalid function call
  │ ╰──────────────────────────^ naga::Function [1]
  │  \n  = Call to [0] is invalid
  = Requires 1 arguments, but 0 are provided

",
    )];

    for (source, expected_err) in cases {
        let module = naga::front::wgsl::parse_str(&source).unwrap();
        let err = valid::Validator::new(Default::default(), valid::Capabilities::all())
            .validate_no_overrides(&module)
            .expect_err("module should be invalid");
        println!("{}", err.emit_to_string(&source));
        assert_eq!(err.emit_to_string(&source), expected_err);
    }
}
