use std::io::Read;
use serde::de;

use super::{Deserializer, DeserializerError as DesErr};

pub struct SeqVisitor<'a, R: Read + 'a> {
    de: &'a mut Deserializer<R>,
    size: usize,
    current: usize,
}

impl<'a, R: Read + 'a> SeqVisitor<'a, R> {
    pub fn new(de: &'a mut Deserializer<R>, size: usize) -> Self {
        SeqVisitor {
            de: de,
            size: size,
            current: 0
        }
    }
}

impl<'a, R: Read + 'a> de::SeqVisitor for SeqVisitor<'a, R> {
    type Error = DesErr;

    fn visit<T>(&mut self) -> Result<Option<T>, Self::Error>
        where T: de::Deserialize
    {
        if self.current >= self.size { return Ok(None) }
        self.current += 1;

        let value = try!(de::Deserialize::deserialize(self.de));
        Ok(Some(value))
    }

     fn end(&mut self) -> Result<(), Self::Error> {
        if self.current < self.size {
            return Err(DesErr::UnexpectedEOF)
        }

        Ok(())
     }
}

pub struct MapVisitor<'a, R: Read + 'a> {
    de: &'a mut Deserializer<R>,
    size: usize,
    current: usize,
}

impl<'a, R: Read> MapVisitor<'a, R> {
    pub fn new(de: &'a mut Deserializer<R>, size: usize) -> Self {
        MapVisitor {
            de: de,
            size: size,
            current: 0
        }
    }
}

impl<'a, R: Read + 'a> de::MapVisitor for MapVisitor<'a, R> {
    type Error = DesErr;

    fn visit_key<K>(&mut self) -> Result<Option<K>, Self::Error>
        where K: de::Deserialize
    {
        if self.current >= self.size { return Ok(None) }

        let value = try!(de::Deserialize::deserialize(self.de));
        Ok(Some(value))
    }

    fn visit_value<V>(&mut self) -> Result<V, Self::Error>
        where V: de::Deserialize
    {
        if self.current >= self.size { return Err(DesErr::UnexpectedEOF) }
        self.current += 1;

        let value = try!(de::Deserialize::deserialize(self.de));
        Ok(value)
    }

    fn end(&mut self) -> Result<(), Self::Error> {
        if self.current < self.size {
            return Err(DesErr::UnexpectedEOF)
        }

        Ok(())
    }
}
