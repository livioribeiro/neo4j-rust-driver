use std::collections::BTreeMap;
use std::string;

use serde::{ser, de};

use super::STRUCTURE_IDENTIFIER;

mod impls;

#[cfg(test)]
mod tests;


#[derive(Clone, PartialEq, PartialOrd, Debug)]
pub enum Value {
    Null,
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(string::String),
    List(self::List),
    Map(self::Map),
    Structure(u8, self::List),
}

pub type List = Vec<Value>;
pub type Map = BTreeMap<String, Value>;

impl Value {
    pub fn is_null(&self) -> bool {
        *self == Value::Null
    }

    pub fn as_boolean(&self) -> Option<bool> {
        match *self {
            Value::Boolean(v) => Some(v),
            _ => None
        }
    }

    pub fn is_boolean(&self) -> bool {
        self.as_boolean().is_some()
    }

    pub fn as_integer(&self) -> Option<i64> {
        match *self {
            Value::Integer(v) => Some(v),
            _ => None
        }
    }

    pub fn is_integer(&self) -> bool {
        self.as_integer().is_some()
    }

    pub fn as_float(&self) -> Option<f64> {
        match *self {
            Value::Float(v) => Some(v),
            _ => None
        }
    }

    pub fn is_float(&self) -> bool {
        self.as_float().is_some()
    }

    pub fn as_string(&self) -> Option<&str> {
        match *self {
            Value::String(ref v) => Some(v),
            _ => None
        }
    }

    pub fn is_string(&self) -> bool {
        self.as_string().is_some()
    }

    pub fn as_list(&self) -> Option<&List> {
        match self {
            &Value::List(ref v) => Some(v),
            _ => None
        }
    }

    pub fn as_list_mut(&mut self) -> Option<&mut List> {
        match self {
            &mut Value::List(ref mut v) => Some(v),
            _ => None
        }
    }

    pub fn is_list(&self) -> bool {
        self.as_list().is_some()
    }

    pub fn as_map(&self) -> Option<&Map> {
        match self {
            &Value::Map(ref v) => Some(v),
            _ => None
        }
    }

    pub fn as_map_mut(&mut self) -> Option<&mut Map> {
        match self {
            &mut Value::Map(ref mut v) => Some(v),
            _ => None
        }
    }

    pub fn is_map(&self) -> bool {
        self.as_map().is_some()
    }

    pub fn as_structure(&self) -> Option<(u8, &List)> {
        match self {
            &Value::Structure(s, ref v) => Some((s, v)),
            _ => None
        }
    }

    pub fn as_structure_mut(&mut self) -> Option<(&mut u8, &mut List)> {
        match self {
            &mut Value::Structure(ref mut s, ref mut v) => Some((s, v)),
            _ => None
        }
    }

    pub fn is_structure(&self) -> bool {
        self.as_structure().is_some()
    }
}

impl ser::Serialize for Value {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: ser::Serializer
    {
        match *self {
            Value::Null => serializer.serialize_unit(),
            Value::Boolean(v) => v.serialize(serializer),
            Value::Integer(v) => v.serialize(serializer),
            Value::Float(v) => v.serialize(serializer),
            Value::String(ref v) => v.serialize(serializer),
            Value::List(ref v) => v.serialize(serializer),
            Value::Map(ref v) => v.serialize(serializer),
            Value::Structure(sig, ref v) => {
                let mut state = try!(serializer.serialize_struct(STRUCTURE_IDENTIFIER, v.len()));
                try!(serializer.serialize_struct_elt(&mut state, "", sig));

                for value in v.iter() {
                    try!(serializer.serialize_struct_elt(&mut state, "", value));
                }

                serializer.serialize_struct_end(state)
            }
        }
    }
}

impl de::Deserialize for Value {
    #[inline]
    fn deserialize<D>(deserializer: &mut D) -> Result<Value, D::Error>
        where D: de::Deserializer,
    {
        struct ValueVisitor;

        impl de::Visitor for ValueVisitor {
            type Value = Value;

            #[inline]
            fn visit_bool<E>(&mut self, value: bool) -> Result<Value, E> {
                Ok(Value::Boolean(value))
            }

            #[inline]
            fn visit_i64<E>(&mut self, value: i64) -> Result<Value, E> {
                Ok(Value::Integer(value))
            }

            #[inline]
            fn visit_u64<E>(&mut self, value: u64) -> Result<Value, E> {
                Ok(Value::Integer(value as i64))
            }

            #[inline]
            fn visit_f64<E>(&mut self, value: f64) -> Result<Value, E> {
                Ok(Value::Float(value))
            }

            #[inline]
            fn visit_str<E>(&mut self, value: &str) -> Result<Value, E>
                where E: de::Error,
            {
                self.visit_string(String::from(value))
            }

            #[inline]
            fn visit_string<E>(&mut self, value: String) -> Result<Value, E> {
                Ok(Value::String(value))
            }

            #[inline]
            fn visit_none<E>(&mut self) -> Result<Value, E> {
                Ok(Value::Null)
            }

            #[inline]
            fn visit_some<D>(
                &mut self,
                deserializer: &mut D
            ) -> Result<Value, D::Error>
                where D: de::Deserializer,
            {
                de::Deserialize::deserialize(deserializer)
            }

            #[inline]
            fn visit_unit<E>(&mut self) -> Result<Value, E> {
                Ok(Value::Null)
            }

            #[inline]
            fn visit_seq<V>(&mut self, visitor: V) -> Result<Value, V::Error>
                where V: de::SeqVisitor,
            {
                let values = try!(de::impls::VecVisitor::new()
                    .visit_seq(visitor));
                Ok(Value::List(values))
            }

            #[inline]
            fn visit_map<V>(&mut self, visitor: V) -> Result<Value, V::Error>
                where V: de::MapVisitor,
            {
                let values = try!(de::impls::BTreeMapVisitor::new().visit_map(visitor));
                Ok(Value::Map(values))
            }
        }

        deserializer.deserialize(ValueVisitor)
    }
}
