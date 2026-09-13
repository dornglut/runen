use std::collections::{BTreeMap, BTreeSet};

/// Verification-only logical record-field identity token.
///
/// The numeric token is implementation machinery for conformance fixtures. It
/// is not source spelling, field position, serialized identity, entity-key
/// identity, or Model ordering.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FieldKey(u32);

impl FieldKey {
    pub const fn new(token: u32) -> Self {
        Self(token)
    }

    const fn storage_key(self) -> u32 {
        self.0
    }
}

/// Verification-only witness distinguishing permitted NaN realizations.
///
/// Model equivalence deliberately erases this token for NaNs of the same exact
/// floating logical type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NaNRealizationId(u32);

impl NaNRealizationId {
    pub const fn new(token: u32) -> Self {
        Self(token)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FloatFormat {
    F16,
    F32,
    F64,
}

impl FloatFormat {
    const fn parameters(self) -> (u32, i32, i32) {
        match self {
            Self::F16 => (11, -14, 15),
            Self::F32 => (24, -126, 127),
            Self::F64 => (53, -1022, 1023),
        }
    }

    fn logical_type(self) -> LogicalType {
        match self {
            Self::F16 => LogicalType::F16,
            Self::F32 => LogicalType::F32,
            Self::F64 => LogicalType::F64,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FixtureError {
    DuplicateFieldKey(FieldKey),
    MissingField(FieldKey),
    ExtraField(FieldKey),
    TypeMismatch {
        expected: LogicalType,
        actual: LogicalType,
    },
    InvalidFloatFiniteMember {
        format: FloatFormat,
        significand: u64,
        exponent: i32,
    },
    MultiplicityOverflow,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordType {
    fields: BTreeMap<u32, LogicalType>,
}

impl RecordType {
    pub fn new<I>(fields: I) -> Result<Self, FixtureError>
    where
        I: IntoIterator<Item = (FieldKey, LogicalType)>,
    {
        let mut map = BTreeMap::new();
        for (key, ty) in fields {
            if map.insert(key.storage_key(), ty).is_some() {
                return Err(FixtureError::DuplicateFieldKey(key));
            }
        }
        Ok(Self { fields: map })
    }

    pub fn field_count(&self) -> usize {
        self.fields.len()
    }

    pub fn field_type(&self, key: FieldKey) -> Option<&LogicalType> {
        self.fields.get(&key.storage_key())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LogicalType {
    Bool,
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    F16,
    F32,
    F64,
    Optional(Box<LogicalType>),
    Record(RecordType),
    Relation(Box<LogicalType>),
    Bag(Box<LogicalType>),
    Sequence(Box<LogicalType>),
}

impl LogicalType {
    pub fn optional(inner: Self) -> Self {
        Self::Optional(Box::new(inner))
    }

    pub fn relation(element: Self) -> Self {
        Self::Relation(Box::new(element))
    }

    pub fn bag(element: Self) -> Self {
        Self::Bag(Box::new(element))
    }

    pub fn sequence(element: Self) -> Self {
        Self::Sequence(Box::new(element))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FloatValue {
    format: FloatFormat,
    kind: FloatKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum FloatKind {
    PositiveZero,
    NegativeZero,
    PositiveInfinity,
    NegativeInfinity,
    Finite {
        negative: bool,
        significand: u64,
        exponent: i32,
    },
    NaN(NaNRealizationId),
}

impl FloatValue {
    pub fn positive_zero(format: FloatFormat) -> Self {
        Self {
            format,
            kind: FloatKind::PositiveZero,
        }
    }

    pub fn negative_zero(format: FloatFormat) -> Self {
        Self {
            format,
            kind: FloatKind::NegativeZero,
        }
    }

    pub fn positive_infinity(format: FloatFormat) -> Self {
        Self {
            format,
            kind: FloatKind::PositiveInfinity,
        }
    }

    pub fn negative_infinity(format: FloatFormat) -> Self {
        Self {
            format,
            kind: FloatKind::NegativeInfinity,
        }
    }

    pub fn nan(format: FloatFormat, witness: NaNRealizationId) -> Self {
        Self {
            format,
            kind: FloatKind::NaN(witness),
        }
    }

    pub fn finite(
        format: FloatFormat,
        negative: bool,
        significand: u64,
        exponent: i32,
    ) -> Result<Self, FixtureError> {
        let (precision, emin, emax) = format.parameters();
        let normal_min = 1_u64 << (precision - 1);
        let normal_max = (1_u64 << precision) - 1;
        let valid_normal = (normal_min..=normal_max).contains(&significand)
            && (emin..=emax).contains(&exponent);
        let valid_subnormal = (1..normal_min).contains(&significand) && exponent == emin;

        if !valid_normal && !valid_subnormal {
            return Err(FixtureError::InvalidFloatFiniteMember {
                format,
                significand,
                exponent,
            });
        }

        Ok(Self {
            format,
            kind: FloatKind::Finite {
                negative,
                significand,
                exponent,
            },
        })
    }

    pub const fn format(&self) -> FloatFormat {
        self.format
    }
}

#[derive(Clone, Debug)]
pub struct Value {
    ty: LogicalType,
    kind: ValueKind,
}

#[derive(Clone, Debug)]
enum ValueKind {
    Bool(bool),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    Float(FloatValue),
    Absent,
    Present(Box<Value>),
    Record(BTreeMap<u32, Value>),
    Relation(RelationValue),
    Bag(BagValue),
    Sequence(SequenceValue),
}

impl Value {
    pub fn logical_type(&self) -> &LogicalType {
        &self.ty
    }

    pub fn bool(value: bool) -> Self {
        Self {
            ty: LogicalType::Bool,
            kind: ValueKind::Bool(value),
        }
    }

    pub fn i8(value: i8) -> Self {
        Self {
            ty: LogicalType::I8,
            kind: ValueKind::I8(value),
        }
    }

    pub fn i16(value: i16) -> Self {
        Self {
            ty: LogicalType::I16,
            kind: ValueKind::I16(value),
        }
    }

    pub fn i32(value: i32) -> Self {
        Self {
            ty: LogicalType::I32,
            kind: ValueKind::I32(value),
        }
    }

    pub fn i64(value: i64) -> Self {
        Self {
            ty: LogicalType::I64,
            kind: ValueKind::I64(value),
        }
    }

    pub fn u8(value: u8) -> Self {
        Self {
            ty: LogicalType::U8,
            kind: ValueKind::U8(value),
        }
    }

    pub fn u16(value: u16) -> Self {
        Self {
            ty: LogicalType::U16,
            kind: ValueKind::U16(value),
        }
    }

    pub fn u32(value: u32) -> Self {
        Self {
            ty: LogicalType::U32,
            kind: ValueKind::U32(value),
        }
    }

    pub fn u64(value: u64) -> Self {
        Self {
            ty: LogicalType::U64,
            kind: ValueKind::U64(value),
        }
    }

    pub fn float(value: FloatValue) -> Self {
        Self {
            ty: value.format.logical_type(),
            kind: ValueKind::Float(value),
        }
    }

    pub fn absent(inner: LogicalType) -> Self {
        Self {
            ty: LogicalType::optional(inner),
            kind: ValueKind::Absent,
        }
    }

    pub fn present(inner: LogicalType, value: Value) -> Result<Self, FixtureError> {
        ensure_type(&inner, &value.ty)?;
        Ok(Self {
            ty: LogicalType::optional(inner),
            kind: ValueKind::Present(Box::new(value)),
        })
    }

    pub fn record<I>(record_type: RecordType, fields: I) -> Result<Self, FixtureError>
    where
        I: IntoIterator<Item = (FieldKey, Value)>,
    {
        let mut values = BTreeMap::new();
        for (key, value) in fields {
            if values.insert(key.storage_key(), value).is_some() {
                return Err(FixtureError::DuplicateFieldKey(key));
            }
        }

        for (key, expected) in &record_type.fields {
            let Some(value) = values.get(key) else {
                return Err(FixtureError::MissingField(FieldKey::new(*key)));
            };
            ensure_type(expected, &value.ty)?;
        }

        if let Some(extra) = values.keys().find(|key| !record_type.fields.contains_key(key)) {
            return Err(FixtureError::ExtraField(FieldKey::new(*extra)));
        }

        Ok(Self {
            ty: LogicalType::Record(record_type),
            kind: ValueKind::Record(values),
        })
    }

    pub fn relation(value: RelationValue) -> Self {
        Self {
            ty: LogicalType::relation(value.element_type.clone()),
            kind: ValueKind::Relation(value),
        }
    }

    pub fn bag(value: BagValue) -> Self {
        Self {
            ty: LogicalType::bag(value.element_type.clone()),
            kind: ValueKind::Bag(value),
        }
    }

    pub fn sequence(value: SequenceValue) -> Self {
        Self {
            ty: LogicalType::sequence(value.element_type.clone()),
            kind: ValueKind::Sequence(value),
        }
    }

    pub fn record_field(&self, key: FieldKey) -> Option<&Value> {
        match &self.kind {
            ValueKind::Record(fields) => fields.get(&key.storage_key()),
            _ => None,
        }
    }

    fn equivalence_key(&self) -> EquivalenceKey {
        match &self.kind {
            ValueKind::Bool(value) => EquivalenceKey::Bool(*value),
            ValueKind::I8(value) => EquivalenceKey::I8(*value),
            ValueKind::I16(value) => EquivalenceKey::I16(*value),
            ValueKind::I32(value) => EquivalenceKey::I32(*value),
            ValueKind::I64(value) => EquivalenceKey::I64(*value),
            ValueKind::U8(value) => EquivalenceKey::U8(*value),
            ValueKind::U16(value) => EquivalenceKey::U16(*value),
            ValueKind::U32(value) => EquivalenceKey::U32(*value),
            ValueKind::U64(value) => EquivalenceKey::U64(*value),
            ValueKind::Float(value) => EquivalenceKey::Float(value.equivalence_key()),
            ValueKind::Absent => EquivalenceKey::Absent,
            ValueKind::Present(value) => {
                EquivalenceKey::Present(Box::new(value.equivalence_key()))
            }
            ValueKind::Record(fields) => EquivalenceKey::Record(
                fields
                    .iter()
                    .map(|(key, value)| (*key, value.equivalence_key()))
                    .collect(),
            ),
            ValueKind::Relation(value) => EquivalenceKey::Relation(value.classes.clone()),
            ValueKind::Bag(value) => EquivalenceKey::Bag(value.classes.clone()),
            ValueKind::Sequence(value) => EquivalenceKey::Sequence(
                value.values.iter().map(Value::equivalence_key).collect(),
            ),
        }
    }
}

impl FloatValue {
    fn equivalence_key(&self) -> FloatEquivalenceKey {
        match &self.kind {
            FloatKind::PositiveZero => FloatEquivalenceKey::PositiveZero,
            FloatKind::NegativeZero => FloatEquivalenceKey::NegativeZero,
            FloatKind::PositiveInfinity => FloatEquivalenceKey::PositiveInfinity,
            FloatKind::NegativeInfinity => FloatEquivalenceKey::NegativeInfinity,
            FloatKind::Finite {
                negative,
                significand,
                exponent,
            } => FloatEquivalenceKey::Finite {
                negative: *negative,
                significand: *significand,
                exponent: *exponent,
            },
            FloatKind::NaN(witness) => {
                let _ = witness;
                FloatEquivalenceKey::NaN
            }
        }
    }
}

fn ensure_type(expected: &LogicalType, actual: &LogicalType) -> Result<(), FixtureError> {
    if expected == actual {
        Ok(())
    } else {
        Err(FixtureError::TypeMismatch {
            expected: expected.clone(),
            actual: actual.clone(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct RelationValue {
    element_type: LogicalType,
    classes: BTreeSet<EquivalenceKey>,
}

impl RelationValue {
    pub fn new<I>(element_type: LogicalType, values: I) -> Result<Self, FixtureError>
    where
        I: IntoIterator<Item = Value>,
    {
        let mut classes = BTreeSet::new();
        for value in values {
            ensure_type(&element_type, &value.ty)?;
            classes.insert(value.equivalence_key());
        }
        Ok(Self {
            element_type,
            classes,
        })
    }

    pub fn element_type(&self) -> &LogicalType {
        &self.element_type
    }

    pub fn class_count(&self) -> usize {
        self.classes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.classes.is_empty()
    }

    pub fn contains_equivalent(&self, value: &Value) -> bool {
        value.ty == self.element_type && self.classes.contains(&value.equivalence_key())
    }
}

#[derive(Clone, Debug)]
pub struct BagValue {
    element_type: LogicalType,
    classes: BTreeMap<EquivalenceKey, u64>,
}

impl BagValue {
    pub fn new<I>(element_type: LogicalType, values: I) -> Result<Self, FixtureError>
    where
        I: IntoIterator<Item = Value>,
    {
        let mut classes = BTreeMap::new();
        for value in values {
            ensure_type(&element_type, &value.ty)?;
            let key = value.equivalence_key();
            let count = classes.entry(key).or_insert(0_u64);
            *count = count
                .checked_add(1)
                .ok_or(FixtureError::MultiplicityOverflow)?;
        }
        Ok(Self {
            element_type,
            classes,
        })
    }

    pub fn element_type(&self) -> &LogicalType {
        &self.element_type
    }

    pub fn class_count(&self) -> usize {
        self.classes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.classes.is_empty()
    }

    pub fn total_multiplicity(&self) -> u64 {
        self.classes.values().copied().sum()
    }

    pub fn multiplicity_of(&self, value: &Value) -> u64 {
        if value.ty != self.element_type {
            return 0;
        }
        self.classes
            .get(&value.equivalence_key())
            .copied()
            .unwrap_or(0)
    }

    pub fn contains_equivalent(&self, value: &Value) -> bool {
        self.multiplicity_of(value) > 0
    }
}

#[derive(Clone, Debug)]
pub struct SequenceValue {
    element_type: LogicalType,
    values: Vec<Value>,
}

impl SequenceValue {
    pub fn new<I>(element_type: LogicalType, values: I) -> Result<Self, FixtureError>
    where
        I: IntoIterator<Item = Value>,
    {
        let values: Vec<Value> = values.into_iter().collect();
        for value in &values {
            ensure_type(&element_type, &value.ty)?;
        }
        Ok(Self {
            element_type,
            values,
        })
    }

    pub fn element_type(&self) -> &LogicalType {
        &self.element_type
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Verification-only positional observation of semantic Sequence order.
    ///
    /// The `usize` carrier is not Runen source indexing syntax or a language
    /// indexing-base rule.
    pub fn at(&self, position: usize) -> Option<&Value> {
        self.values.get(position)
    }
}

pub fn model_equivalent(left: &Value, right: &Value) -> bool {
    left.ty == right.ty && left.equivalence_key() == right.equivalence_key()
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum EquivalenceKey {
    Bool(bool),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    Float(FloatEquivalenceKey),
    Absent,
    Present(Box<EquivalenceKey>),
    Record(BTreeMap<u32, EquivalenceKey>),
    Relation(BTreeSet<EquivalenceKey>),
    Bag(BTreeMap<EquivalenceKey, u64>),
    Sequence(Vec<EquivalenceKey>),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum FloatEquivalenceKey {
    PositiveZero,
    NegativeZero,
    PositiveInfinity,
    NegativeInfinity,
    Finite {
        negative: bool,
        significand: u64,
        exponent: i32,
    },
    NaN,
}

pub(crate) fn relation_from_support(bag: &BagValue) -> RelationValue {
    RelationValue {
        element_type: bag.element_type.clone(),
        classes: bag.classes.keys().cloned().collect(),
    }
}
