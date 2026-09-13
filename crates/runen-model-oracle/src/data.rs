use std::collections::{BTreeMap, BTreeSet};

use num_bigint::BigUint;

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
    ProjectionRequiresRecord {
        actual: LogicalType,
    },
    ProjectionFieldNotFound(FieldKey),
    FilterRequiresRecord {
        actual: LogicalType,
    },
    FilterFieldNotFound(FieldKey),
    JoinLeftRequiresRecord {
        actual: LogicalType,
    },
    JoinRightRequiresRecord {
        actual: LogicalType,
    },
    JoinOverlappingField(FieldKey),
    JoinLeftFieldNotFound(FieldKey),
    JoinRightFieldNotFound(FieldKey),
    JoinFieldTypeMismatch {
        left: LogicalType,
        right: LogicalType,
    },
    GroupingRequiresRecord {
        actual: LogicalType,
    },
    GroupingFieldNotFound(FieldKey),
    InvalidFloatFiniteMember {
        format: FloatFormat,
        significand: u64,
        exponent: i32,
    },
    ZeroMultiplicity,
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
    Cardinality,
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

/// Verification-only semantic floating member fixture.
///
/// This type intentionally does not implement Rust `PartialEq`/`Eq`: Model
/// value equivalence is the separately owned relation exposed through
/// [`model_equivalent`] after wrapping the fixture with [`Value::float`].
#[derive(Clone)]
pub struct FloatValue {
    format: FloatFormat,
    kind: FloatKind,
}

#[derive(Clone, PartialEq, Eq)]
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
        let valid_normal =
            (normal_min..=normal_max).contains(&significand) && (emin..=emax).contains(&exponent);
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

#[derive(Clone)]
pub struct Value {
    ty: LogicalType,
    kind: ValueKind,
}

#[derive(Clone)]
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
    Cardinality(BigUint),
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

    /// Construct a verification-only Cardinality witness from a bounded Rust
    /// carrier. The `u128` input is only fixture machinery and is not a Model
    /// Cardinality maximum.
    pub fn cardinality_from_u128(value: u128) -> Self {
        Self::cardinality_from_biguint(BigUint::from(value))
    }

    pub(crate) fn cardinality_from_biguint(value: BigUint) -> Self {
        Self {
            ty: LogicalType::Cardinality,
            kind: ValueKind::Cardinality(value),
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

        if let Some(extra) = values
            .keys()
            .find(|key| !record_type.fields.contains_key(key))
        {
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
            ValueKind::Cardinality(value) => EquivalenceKey::Cardinality(value.clone()),
            ValueKind::Absent => EquivalenceKey::Absent,
            ValueKind::Present(value) => EquivalenceKey::Present(Box::new(value.equivalence_key())),
            ValueKind::Record(fields) => EquivalenceKey::Record(
                fields
                    .iter()
                    .map(|(key, value)| (*key, value.equivalence_key()))
                    .collect(),
            ),
            ValueKind::Relation(value) => EquivalenceKey::Relation(value.classes.clone()),
            ValueKind::Bag(value) => EquivalenceKey::Bag(value.classes.clone()),
            ValueKind::Sequence(value) => {
                EquivalenceKey::Sequence(value.values.iter().map(Value::equivalence_key).collect())
            }
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

#[derive(Clone)]
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

#[derive(Clone)]
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

    /// Construct a finite verification Bag from explicit class-occurrence
    /// multiplicities without allocating one Rust value per occurrence.
    ///
    /// The `u64` multiplicity carrier is verification machinery, not a Model
    /// multiplicity or Cardinality bound. Model-equivalent supplied values are
    /// merged with checked fixture arithmetic.
    pub fn from_multiplicities<I>(
        element_type: LogicalType,
        values: I,
    ) -> Result<Self, FixtureError>
    where
        I: IntoIterator<Item = (Value, u64)>,
    {
        let mut classes = BTreeMap::new();
        for (value, multiplicity) in values {
            ensure_type(&element_type, &value.ty)?;
            if multiplicity == 0 {
                return Err(FixtureError::ZeroMultiplicity);
            }
            let key = value.equivalence_key();
            let count = classes.entry(key).or_insert(0_u64);
            *count = count
                .checked_add(multiplicity)
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

    /// Return the total occurrence count only when it fits the bounded `u64`
    /// fixture carrier. This is not the semantic `bag_cardinality` operation.
    pub fn total_multiplicity(&self) -> Result<u64, FixtureError> {
        self.classes.values().try_fold(0_u64, |total, multiplicity| {
            total
                .checked_add(*multiplicity)
                .ok_or(FixtureError::MultiplicityOverflow)
        })
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

#[derive(Clone)]
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

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
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
    Cardinality(BigUint),
    Absent,
    Present(Box<EquivalenceKey>),
    Record(BTreeMap<u32, EquivalenceKey>),
    Relation(BTreeSet<EquivalenceKey>),
    Bag(BTreeMap<EquivalenceKey, u64>),
    Sequence(Vec<EquivalenceKey>),
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
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

pub(crate) fn exact_bag_cardinality(bag: &BagValue) -> Value {
    let total = bag
        .classes
        .values()
        .fold(BigUint::from(0_u8), |total, multiplicity| {
            total + BigUint::from(*multiplicity)
        });
    Value::cardinality_from_biguint(total)
}

pub(crate) fn relation_from_support(bag: &BagValue) -> RelationValue {
    RelationValue {
        element_type: bag.element_type.clone(),
        classes: bag.classes.keys().cloned().collect(),
    }
}

pub(crate) fn project_record_fields(
    bag: &BagValue,
    retained_fields: &[FieldKey],
) -> Result<BagValue, FixtureError> {
    let LogicalType::Record(record_type) = &bag.element_type else {
        return Err(FixtureError::ProjectionRequiresRecord {
            actual: bag.element_type.clone(),
        });
    };

    let mut output_fields = BTreeMap::new();
    for key in retained_fields {
        let storage_key = key.storage_key();
        let Some(field_type) = record_type.fields.get(&storage_key) else {
            return Err(FixtureError::ProjectionFieldNotFound(*key));
        };
        output_fields
            .entry(storage_key)
            .or_insert_with(|| field_type.clone());
    }

    let output_record_type = RecordType {
        fields: output_fields.clone(),
    };
    let mut output_classes = BTreeMap::new();

    for (class, multiplicity) in &bag.classes {
        let EquivalenceKey::Record(fields) = class else {
            unreachable!("validated Bag<Record> must contain record equivalence keys");
        };

        let mut projected_fields = BTreeMap::new();
        for key in output_fields.keys() {
            let Some(value) = fields.get(key) else {
                unreachable!("validated record equivalence key must contain every declared field");
            };
            projected_fields.insert(*key, value.clone());
        }

        let count = output_classes
            .entry(EquivalenceKey::Record(projected_fields))
            .or_insert(0_u64);
        *count = count
            .checked_add(*multiplicity)
            .ok_or(FixtureError::MultiplicityOverflow)?;
    }

    Ok(BagValue {
        element_type: LogicalType::Record(output_record_type),
        classes: output_classes,
    })
}

pub(crate) fn filter_record_field_equivalent(
    bag: &BagValue,
    field: FieldKey,
    value: &Value,
) -> Result<BagValue, FixtureError> {
    let LogicalType::Record(record_type) = &bag.element_type else {
        return Err(FixtureError::FilterRequiresRecord {
            actual: bag.element_type.clone(),
        });
    };

    let storage_key = field.storage_key();
    let Some(field_type) = record_type.fields.get(&storage_key) else {
        return Err(FixtureError::FilterFieldNotFound(field));
    };
    ensure_type(field_type, &value.ty)?;

    let target = value.equivalence_key();
    let mut output_classes = BTreeMap::new();
    for (class, multiplicity) in &bag.classes {
        let EquivalenceKey::Record(fields) = class else {
            unreachable!("validated Bag<Record> must contain record equivalence keys");
        };
        let Some(field_value) = fields.get(&storage_key) else {
            unreachable!("validated record equivalence key must contain every declared field");
        };
        if field_value == &target {
            output_classes.insert(class.clone(), *multiplicity);
        }
    }

    Ok(BagValue {
        element_type: bag.element_type.clone(),
        classes: output_classes,
    })
}

pub(crate) fn join_record_fields_equivalent(
    left: &BagValue,
    left_field: FieldKey,
    right: &BagValue,
    right_field: FieldKey,
) -> Result<BagValue, FixtureError> {
    let LogicalType::Record(left_type) = &left.element_type else {
        return Err(FixtureError::JoinLeftRequiresRecord {
            actual: left.element_type.clone(),
        });
    };
    let LogicalType::Record(right_type) = &right.element_type else {
        return Err(FixtureError::JoinRightRequiresRecord {
            actual: right.element_type.clone(),
        });
    };

    if let Some(key) = left_type
        .fields
        .keys()
        .find(|key| right_type.fields.contains_key(key))
    {
        return Err(FixtureError::JoinOverlappingField(FieldKey::new(*key)));
    }

    let left_storage_key = left_field.storage_key();
    let right_storage_key = right_field.storage_key();
    let Some(left_field_type) = left_type.fields.get(&left_storage_key) else {
        return Err(FixtureError::JoinLeftFieldNotFound(left_field));
    };
    let Some(right_field_type) = right_type.fields.get(&right_storage_key) else {
        return Err(FixtureError::JoinRightFieldNotFound(right_field));
    };
    if left_field_type != right_field_type {
        return Err(FixtureError::JoinFieldTypeMismatch {
            left: left_field_type.clone(),
            right: right_field_type.clone(),
        });
    }

    let mut output_fields = left_type.fields.clone();
    output_fields.extend(right_type.fields.clone());
    let output_record_type = RecordType {
        fields: output_fields,
    };
    let mut output_classes = BTreeMap::new();

    for (left_class, left_multiplicity) in &left.classes {
        let EquivalenceKey::Record(left_fields) = left_class else {
            unreachable!("validated Bag<Record> must contain record equivalence keys");
        };
        let Some(left_value) = left_fields.get(&left_storage_key) else {
            unreachable!("validated record equivalence key must contain every declared field");
        };

        for (right_class, right_multiplicity) in &right.classes {
            let EquivalenceKey::Record(right_fields) = right_class else {
                unreachable!("validated Bag<Record> must contain record equivalence keys");
            };
            let Some(right_value) = right_fields.get(&right_storage_key) else {
                unreachable!("validated record equivalence key must contain every declared field");
            };
            if left_value != right_value {
                continue;
            }

            let mut merged_fields = left_fields.clone();
            merged_fields.extend(right_fields.clone());
            let contribution = left_multiplicity
                .checked_mul(*right_multiplicity)
                .ok_or(FixtureError::MultiplicityOverflow)?;
            if output_classes
                .insert(EquivalenceKey::Record(merged_fields), contribution)
                .is_some()
            {
                unreachable!("disjoint complete record schemas make join class pairs injective");
            }
        }
    }

    Ok(BagValue {
        element_type: LogicalType::Record(output_record_type),
        classes: output_classes,
    })
}

pub(crate) fn group_record_fields(
    bag: &BagValue,
    grouping_fields: &[FieldKey],
) -> Result<RelationValue, FixtureError> {
    let LogicalType::Record(record_type) = &bag.element_type else {
        return Err(FixtureError::GroupingRequiresRecord {
            actual: bag.element_type.clone(),
        });
    };

    let mut retained_keys = BTreeSet::new();
    for field in grouping_fields {
        let storage_key = field.storage_key();
        if !record_type.fields.contains_key(&storage_key) {
            return Err(FixtureError::GroupingFieldNotFound(*field));
        }
        retained_keys.insert(storage_key);
    }

    let mut groups: BTreeMap<EquivalenceKey, BTreeMap<EquivalenceKey, u64>> = BTreeMap::new();
    for (class, multiplicity) in &bag.classes {
        let EquivalenceKey::Record(fields) = class else {
            unreachable!("validated Bag<Record> must contain record equivalence keys");
        };

        let mut projected_fields = BTreeMap::new();
        for key in &retained_keys {
            let Some(value) = fields.get(key) else {
                unreachable!("validated record equivalence key must contain every declared field");
            };
            projected_fields.insert(*key, value.clone());
        }

        let group = groups
            .entry(EquivalenceKey::Record(projected_fields))
            .or_default();
        if group.insert(class.clone(), *multiplicity).is_some() {
            unreachable!("each input record equivalence class belongs to one grouping block");
        }
    }

    let mut output_classes = BTreeSet::new();
    for group in groups.into_values() {
        if !output_classes.insert(EquivalenceKey::Bag(group)) {
            unreachable!("distinct grouping blocks must remain distinct Bag equivalence classes");
        }
    }

    Ok(RelationValue {
        element_type: LogicalType::bag(bag.element_type.clone()),
        classes: output_classes,
    })
}
