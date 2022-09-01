use serde::Serialize;
use soroban_env_host::xdr::{
    ScSpecEntry, ScSpecFunctionInputV0, ScSpecTypeDef, ScSpecUdtStructFieldV0, ScSpecUdtUnionCaseV0,
};

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StructField {
    name: String,
    value: Type,
}

impl From<&ScSpecUdtStructFieldV0> for StructField {
    fn from(f: &ScSpecUdtStructFieldV0) -> Self {
        StructField {
            name: f.name.to_string_lossy(),
            value: (&f.type_).into(),
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FunctionInput {
    name: String,
    value: Type,
}

impl From<&ScSpecFunctionInputV0> for FunctionInput {
    fn from(f: &ScSpecFunctionInputV0) -> Self {
        FunctionInput {
            name: f.name.to_string_lossy(),
            value: (&f.type_).into(),
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnionCase {
    name: String,
    values: Vec<Type>,
}

impl From<&ScSpecUdtUnionCaseV0> for UnionCase {
    fn from(c: &ScSpecUdtUnionCaseV0) -> Self {
        UnionCase {
            name: c.name.to_string_lossy(),
            values: c.type_.as_ref().map(Type::from).into_iter().collect(),
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(tag = "type")]
#[serde(rename_all = "camelCase")]
pub enum Type {
    Val,
    U64,
    I64,
    U32,
    I32,
    Bool,
    Symbol,
    Bitset,
    Status,
    Bytes,
    BigInt,
    Map { key: Box<Type>, value: Box<Type> },
    Option { value: Box<Type> },
    Result { value: Box<Type>, error: Box<Type> },
    Set { element: Box<Type> },
    Vec { element: Box<Type> },
    BytesN { n: u32 },
    Tuple { elements: Vec<Type> },
    Custom { name: String },
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(tag = "type")]
#[serde(rename_all = "camelCase")]
pub enum Entry {
    Function {
        name: String,
        inputs: Vec<FunctionInput>,
        outputs: Vec<Type>,
    },
    Struct {
        name: String,
        fields: Vec<StructField>,
    },
    Union {
        name: String,
        cases: Vec<UnionCase>,
    },
}

impl From<&ScSpecTypeDef> for Type {
    fn from(spec: &ScSpecTypeDef) -> Self {
        match spec {
            ScSpecTypeDef::Map(map) => Type::Map {
                key: Box::new(Type::from(map.key_type.as_ref())),
                value: Box::new(Type::from(map.value_type.as_ref())),
            },
            ScSpecTypeDef::Option(opt) => Type::Option {
                value: Box::new(Type::from(opt.value_type.as_ref())),
            },
            ScSpecTypeDef::Result(res) => Type::Result {
                value: Box::new(Type::from(res.ok_type.as_ref())),
                error: Box::new(Type::from(res.error_type.as_ref())),
            },
            ScSpecTypeDef::Set(set) => Type::Set {
                element: Box::new(Type::from(set.element_type.as_ref())),
            },
            ScSpecTypeDef::Tuple(tuple) => Type::Tuple {
                elements: tuple.value_types.iter().map(Type::from).collect(),
            },
            ScSpecTypeDef::Vec(vec) => Type::Vec {
                element: Box::new(Type::from(vec.element_type.as_ref())),
            },
            ScSpecTypeDef::Udt(udt) => Type::Custom {
                name: udt.name.to_string_lossy(),
            },
            ScSpecTypeDef::BytesN(b) => Type::BytesN { n: b.n },
            ScSpecTypeDef::Val => Type::Val,
            ScSpecTypeDef::U64 => Type::U64,
            ScSpecTypeDef::I64 => Type::I64,
            ScSpecTypeDef::U32 => Type::U32,
            ScSpecTypeDef::I32 => Type::I32,
            ScSpecTypeDef::Bool => Type::Bool,
            ScSpecTypeDef::Symbol => Type::Symbol,
            ScSpecTypeDef::Bitset => Type::Bitset,
            ScSpecTypeDef::Status => Type::Status,
            ScSpecTypeDef::Bytes => Type::Bytes,
            ScSpecTypeDef::BigInt => Type::BigInt,
        }
    }
}

impl From<&ScSpecEntry> for Entry {
    fn from(spec: &ScSpecEntry) -> Self {
        match spec {
            ScSpecEntry::FunctionV0(f) => Entry::Function {
                name: f.name.to_string_lossy(),
                inputs: f.inputs.iter().map(FunctionInput::from).collect(),
                outputs: f.outputs.iter().map(Type::from).collect(),
            },
            ScSpecEntry::UdtStructV0(s) => Entry::Struct {
                name: s.name.to_string_lossy(),
                fields: s.fields.iter().map(StructField::from).collect(),
            },
            ScSpecEntry::UdtUnionV0(u) => Entry::Union {
                name: u.name.to_string_lossy(),
                cases: u.cases.iter().map(UnionCase::from).collect(),
            },
        }
    }
}