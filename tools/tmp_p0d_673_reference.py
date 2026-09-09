from pathlib import Path

path = Path("crates/runen-reference/src/interprocedural.rs")
text = path.read_text()


def once(old: str, new: str) -> None:
    global text
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"expected one occurrence, found {count}: {old[:120]!r}")
    text = text.replace(old, new, 1)


once(
    """    F16(RuntimeFloatValue),
    F32(RuntimeFloatValue),
    F64(RuntimeFloatValue),
    RawPointer(RawPointerValue),""",
    """    F16(RuntimeFloatValue),
    F32(RuntimeFloatValue),
    F64(RuntimeFloatValue),
    Function(FunctionId),
    RawPointer(RawPointerValue),""",
)

once(
    """            Self::F16(value) => ObservedValue::F16(value.into_observed()),
            Self::F32(value) => ObservedValue::F32(value.into_observed()),
            Self::F64(value) => ObservedValue::F64(value.into_observed()),
            Self::TrackedFixture(value) => ObservedValue::TrackedFixture(value),""",
    """            Self::F16(value) => ObservedValue::F16(value.into_observed()),
            Self::F32(value) => ObservedValue::F32(value.into_observed()),
            Self::F64(value) => ObservedValue::F64(value.into_observed()),
            Self::Function(function) => ObservedValue::Function(function),
            Self::TrackedFixture(value) => ObservedValue::TrackedFixture(value),""",
)

old_call = '''                Terminator::Call {
                    function,
                    arguments,
                    destination,
                    target,
                } => {
                    let mut values = Vec::with_capacity(arguments.len());
                    for argument in &arguments {
                        match self.evaluate_operand(frame_index, argument) {
                            Ok(value) => values.push(value),
                            Err(kind) => {
                                return Err(UndefinedBehavior {
                                    kind,
                                    verification_events: self.verification_events,
                                });
                            }
                        }
                    }
                    let callee = self.create_frame(
                        function,
                        values,
                        Some(Continuation {
                            destination,
                            target,
                        }),
                    );
                    self.frames.push(callee);
                }
'''
new_call = '''                Terminator::Call {
                    function,
                    arguments,
                    destination,
                    target,
                } => {
                    if let Err(kind) = self.start_call(
                        frame_index,
                        function,
                        &arguments,
                        destination,
                        target,
                    ) {
                        return Err(UndefinedBehavior {
                            kind,
                            verification_events: self.verification_events,
                        });
                    }
                }
                Terminator::IndirectCall {
                    callable: _,
                    callee,
                    arguments,
                    destination,
                    target,
                } => {
                    let function = match self.evaluate_operand(frame_index, &callee) {
                        Ok(RuntimeValue::Function(function)) => function,
                        Ok(_) => unreachable!(
                            "validated indirect call callee has its exact callable type"
                        ),
                        Err(kind) => {
                            return Err(UndefinedBehavior {
                                kind,
                                verification_events: self.verification_events,
                            });
                        }
                    };
                    if let Err(kind) = self.start_call(
                        frame_index,
                        function,
                        &arguments,
                        destination,
                        target,
                    ) {
                        return Err(UndefinedBehavior {
                            kind,
                            verification_events: self.verification_events,
                        });
                    }
                }
'''
once(old_call, new_call)

helper = '''    fn start_call(
        &mut self,
        frame_index: usize,
        function: FunctionId,
        arguments: &[Operand],
        destination: Option<Place>,
        target: BasicBlockId,
    ) -> Result<(), UndefinedBehaviorKind> {
        let mut values = Vec::with_capacity(arguments.len());
        for argument in arguments {
            values.push(self.evaluate_operand(frame_index, argument)?);
        }
        let callee = self.create_frame(
            function,
            values,
            Some(Continuation {
                destination,
                target,
            }),
        );
        self.frames.push(callee);
        Ok(())
    }

'''
once("    fn create_frame(\n", helper + "    fn create_frame(\n")

once(
    """        match operand {
            Operand::Constant(value) => Ok(RuntimeValue::from_constant(value)),
            Operand::Move(src) => {""",
    """        match operand {
            Operand::Constant(value) => Ok(RuntimeValue::from_constant(value)),
            Operand::FunctionValue(function) => Ok(RuntimeValue::Function(*function)),
            Operand::Move(src) => {""",
)

once(
    """        (TypeKind::Scalar(ScalarType::F64), ObjectState::Leaf(leaf), RuntimeValue::F64(value)) => {
            *leaf = LeafState::Live(RuntimeValue::F64(value));
        }
        (
            TypeKind::Scalar(ScalarType::RawPointer(_)),""",
    """        (TypeKind::Scalar(ScalarType::F64), ObjectState::Leaf(leaf), RuntimeValue::F64(value)) => {
            *leaf = LeafState::Live(RuntimeValue::F64(value));
        }
        (
            TypeKind::Scalar(ScalarType::Callable(_)),
            ObjectState::Leaf(leaf),
            RuntimeValue::Function(function),
        ) => *leaf = LeafState::Live(RuntimeValue::Function(function)),
        (
            TypeKind::Scalar(ScalarType::RawPointer(_)),""",
)

path.write_text(text)
